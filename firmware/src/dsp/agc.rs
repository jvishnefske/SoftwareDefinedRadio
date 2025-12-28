//! Automatic Gain Control
//!
//! Provides AGC algorithms for maintaining consistent audio levels.
//! Supports fast attack and slow decay for natural-sounding AGC.

#[cfg(feature = "embedded")]
use micromath::F32Ext;

/// AGC configuration
#[derive(Clone, Copy, Debug)]
pub struct AgcConfig {
    /// Attack time constant in samples
    pub attack_samples: u32,
    /// Decay time constant in samples
    pub decay_samples: u32,
    /// Target output level (0.0 to 1.0)
    pub target_level: f32,
    /// Maximum gain in dB
    pub max_gain_db: f32,
    /// Minimum gain in dB
    pub min_gain_db: f32,
    /// Hang time in samples (delay before decay starts)
    pub hang_samples: u32,
}

impl AgcConfig {
    /// Create AGC config from time constants in milliseconds
    #[must_use]
    pub fn from_ms(sample_rate: u32, attack_ms: u32, decay_ms: u32) -> Self {
        let samples_per_ms = sample_rate / 1000;
        Self {
            attack_samples: attack_ms * samples_per_ms,
            decay_samples: decay_ms * samples_per_ms,
            target_level: 0.5,
            max_gain_db: 60.0,
            min_gain_db: -20.0,
            hang_samples: 100 * samples_per_ms,
        }
    }

    /// Calculate attack coefficient
    fn attack_coeff(&self) -> f32 {
        if self.attack_samples == 0 {
            1.0
        } else {
            1.0 - (-1.0 / self.attack_samples as f32).exp()
        }
    }

    /// Calculate decay coefficient
    fn decay_coeff(&self) -> f32 {
        if self.decay_samples == 0 {
            1.0
        } else {
            1.0 - (-1.0 / self.decay_samples as f32).exp()
        }
    }
}

impl Default for AgcConfig {
    fn default() -> Self {
        Self {
            attack_samples: 480,  // 10ms at 48kHz
            decay_samples: 24000, // 500ms at 48kHz
            target_level: 0.5,
            max_gain_db: 60.0,
            min_gain_db: -20.0,
            hang_samples: 4800, // 100ms
        }
    }
}

/// AGC state
#[derive(Clone, Copy, Debug)]
pub struct Agc {
    config: AgcConfig,
    /// Current gain (linear)
    gain: f32,
    /// Envelope follower output
    envelope: f32,
    /// Hang timer (samples remaining)
    hang_counter: u32,
    /// Attack coefficient (cached)
    attack_coeff: f32,
    /// Decay coefficient (cached)
    decay_coeff: f32,
}

impl Agc {
    /// Create a new AGC processor
    #[must_use]
    pub fn new(config: AgcConfig) -> Self {
        let attack_coeff = config.attack_coeff();
        let decay_coeff = config.decay_coeff();

        Self {
            config,
            gain: 1.0,
            envelope: 0.0,
            hang_counter: 0,
            attack_coeff,
            decay_coeff,
        }
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        let abs_input = input.abs();

        // Update envelope follower
        if abs_input > self.envelope {
            // Attack - envelope follows signal quickly
            self.envelope += self.attack_coeff * (abs_input - self.envelope);
            self.hang_counter = self.config.hang_samples;
        } else if self.hang_counter > 0 {
            // Hang - hold envelope
            self.hang_counter -= 1;
        } else {
            // Decay - envelope falls slowly
            self.envelope += self.decay_coeff * (abs_input - self.envelope);
        }

        // Calculate desired gain
        let desired_gain = if self.envelope > 0.0001 {
            self.config.target_level / self.envelope
        } else {
            self.db_to_linear(self.config.max_gain_db)
        };

        // Clamp gain to limits
        let max_gain = self.db_to_linear(self.config.max_gain_db);
        let min_gain = self.db_to_linear(self.config.min_gain_db);
        let clamped_gain = desired_gain.clamp(min_gain, max_gain);

        // Smooth gain changes
        if clamped_gain < self.gain {
            self.gain += self.attack_coeff * (clamped_gain - self.gain);
        } else {
            self.gain += self.decay_coeff * (clamped_gain - self.gain);
        }

        // Apply gain
        input * self.gain
    }

    /// Process a block of samples in-place
    pub fn process_block(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Get current gain in dB
    #[must_use]
    pub fn gain_db(&self) -> f32 {
        20.0 * self.gain.log10()
    }

    /// Get current envelope level
    #[must_use]
    pub fn envelope(&self) -> f32 {
        self.envelope
    }

    /// Reset AGC state
    pub fn reset(&mut self) {
        self.gain = 1.0;
        self.envelope = 0.0;
        self.hang_counter = 0;
    }

    /// Update configuration
    pub fn set_config(&mut self, config: AgcConfig) {
        self.config = config;
        self.attack_coeff = config.attack_coeff();
        self.decay_coeff = config.decay_coeff();
    }

    fn db_to_linear(&self, db: f32) -> f32 {
        10.0f32.powf(db / 20.0)
    }
}

impl Default for Agc {
    fn default() -> Self {
        Self::new(AgcConfig::default())
    }
}

/// S-meter reading derived from AGC
#[derive(Clone, Copy, Debug)]
pub struct SMeter {
    /// Current S-meter value (0-9, then +10, +20, etc.)
    value: f32,
    /// Smoothing filter
    smoothed: f32,
    /// Smoothing coefficient
    alpha: f32,
}

impl SMeter {
    /// Create a new S-meter
    #[must_use]
    pub const fn new() -> Self {
        Self {
            value: 0.0,
            smoothed: 0.0,
            alpha: 0.1,
        }
    }

    /// Update from AGC gain (inverse relationship)
    pub fn update_from_agc(&mut self, agc: &Agc) {
        // S-meter is inversely related to AGC gain
        // S9 = -73 dBm reference, 6 dB per S-unit
        let gain_db = agc.gain_db();
        let signal_db = -gain_db; // Higher signal = lower gain needed

        // Map to S-units (approximate)
        // S1 = -121 dBm, S9 = -73 dBm, +60 = -13 dBm
        let s_value = (signal_db + 121.0) / 6.0;
        self.value = s_value.clamp(0.0, 15.0); // S0 to S9+60

        // Apply smoothing
        self.smoothed += self.alpha * (self.value - self.smoothed);
    }

    /// Update from raw signal level
    pub fn update_from_level(&mut self, level: f32) {
        let db = 20.0 * (level.max(0.00001)).log10();
        let s_value = (db + 80.0) / 6.0; // Approximate mapping
        self.value = s_value.clamp(0.0, 15.0);
        self.smoothed += self.alpha * (self.value - self.smoothed);
    }

    /// Get smoothed S-meter value
    #[must_use]
    pub fn value(&self) -> f32 {
        self.smoothed
    }

    /// Get S-meter as integer (S-units)
    #[must_use]
    pub fn s_units(&self) -> u8 {
        self.smoothed.min(9.0) as u8
    }

    /// Get dB over S9 (0 if below S9)
    #[must_use]
    pub fn db_over_s9(&self) -> u8 {
        if self.smoothed > 9.0 {
            ((self.smoothed - 9.0) * 6.0) as u8
        } else {
            0
        }
    }

    /// Get as percentage (0-100)
    #[must_use]
    pub fn as_percent(&self) -> u8 {
        ((self.smoothed / 15.0) * 100.0) as u8
    }
}

impl Default for SMeter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for SMeter {
    fn format(&self, f: defmt::Formatter) {
        let s = self.s_units();
        let db = self.db_over_s9();
        if db > 0 {
            defmt::write!(f, "S9+{}", db);
        } else {
            defmt::write!(f, "S{}", s);
        }
    }
}
