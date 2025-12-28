//! Automatic Gain Control (AGC) and S-Meter.
//!
//! Provides automatic level control for consistent audio output
//! and signal strength measurement for display.

#[allow(unused_imports)]
use micromath::F32Ext;

/// AGC configuration parameters.
#[derive(Clone, Copy, Debug)]
pub struct AgcConfig {
    /// Target output level (0.0 to 1.0)
    pub target_level: f32,
    /// Attack time constant in milliseconds
    pub attack_ms: f32,
    /// Decay time constant in milliseconds
    pub decay_ms: f32,
    /// Hang time before decay starts, in milliseconds
    pub hang_ms: f32,
    /// Minimum gain (prevents over-amplification of noise)
    pub min_gain: f32,
    /// Maximum gain
    pub max_gain: f32,
}

impl Default for AgcConfig {
    fn default() -> Self {
        Self {
            target_level: 0.3,
            attack_ms: 5.0,
            decay_ms: 500.0,
            hang_ms: 200.0,
            min_gain: 0.01,
            max_gain: 1000.0,
        }
    }
}

impl AgcConfig {
    /// Fast AGC preset for CW.
    #[must_use]
    pub const fn fast() -> Self {
        Self {
            target_level: 0.3,
            attack_ms: 2.0,
            decay_ms: 100.0,
            hang_ms: 50.0,
            min_gain: 0.01,
            max_gain: 1000.0,
        }
    }

    /// Medium AGC preset for SSB.
    #[must_use]
    pub const fn medium() -> Self {
        Self {
            target_level: 0.3,
            attack_ms: 5.0,
            decay_ms: 500.0,
            hang_ms: 200.0,
            min_gain: 0.01,
            max_gain: 1000.0,
        }
    }

    /// Slow AGC preset for AM broadcast.
    #[must_use]
    pub const fn slow() -> Self {
        Self {
            target_level: 0.3,
            attack_ms: 10.0,
            decay_ms: 2000.0,
            hang_ms: 500.0,
            min_gain: 0.01,
            max_gain: 1000.0,
        }
    }
}

/// Automatic Gain Control.
///
/// Maintains consistent output level by adjusting gain based on input signal level.
/// Features attack/decay time constants and hang time.
#[derive(Clone, Debug)]
pub struct Agc {
    config: AgcConfig,
    /// Current gain
    gain: f32,
    /// Attack coefficient (per sample)
    attack_coeff: f32,
    /// Decay coefficient (per sample)
    decay_coeff: f32,
    /// Hang counter (samples remaining)
    hang_counter: u32,
    /// Hang samples
    hang_samples: u32,
    /// Sample rate
    sample_rate: f32,
    /// Peak detector for input level
    peak_level: f32,
    /// Peak detector coefficient
    peak_coeff: f32,
}

impl Agc {
    /// Create a new AGC with given configuration.
    #[must_use]
    pub fn new(sample_rate: f32, config: AgcConfig) -> Self {
        let attack_coeff = Self::time_to_coeff(config.attack_ms, sample_rate);
        let decay_coeff = Self::time_to_coeff(config.decay_ms, sample_rate);
        let hang_samples = (config.hang_ms * sample_rate / 1000.0) as u32;

        Self {
            config,
            gain: 1.0,
            attack_coeff,
            decay_coeff,
            hang_counter: 0,
            hang_samples,
            sample_rate,
            peak_level: 0.0,
            peak_coeff: Self::time_to_coeff(10.0, sample_rate), // 10ms peak detector
        }
    }

    /// Convert time constant (ms) to exponential coefficient.
    fn time_to_coeff(time_ms: f32, sample_rate: f32) -> f32 {
        if time_ms <= 0.0 {
            1.0
        } else {
            1.0 - (-1.0 / (time_ms * sample_rate / 1000.0)).exp()
        }
    }

    /// Process a single sample.
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let input_abs = input.abs();

        // Update peak detector
        if input_abs > self.peak_level {
            self.peak_level = input_abs;
        } else {
            self.peak_level += self.peak_coeff * (input_abs - self.peak_level);
        }

        // Calculate desired gain
        let desired_gain = if self.peak_level > 1e-10 {
            (self.config.target_level / self.peak_level).clamp(self.config.min_gain, self.config.max_gain)
        } else {
            self.config.max_gain
        };

        // Update gain with attack/decay/hang
        if desired_gain < self.gain {
            // Attack: reduce gain quickly
            self.gain += self.attack_coeff * (desired_gain - self.gain);
            self.hang_counter = self.hang_samples;
        } else if self.hang_counter > 0 {
            // Hang: hold gain
            self.hang_counter -= 1;
        } else {
            // Decay: increase gain slowly
            self.gain += self.decay_coeff * (desired_gain - self.gain);
        }

        input * self.gain
    }

    /// Get current gain.
    #[must_use]
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Get current peak level (before AGC).
    #[must_use]
    pub fn peak_level(&self) -> f32 {
        self.peak_level
    }

    /// Update configuration.
    pub fn set_config(&mut self, config: AgcConfig) {
        self.attack_coeff = Self::time_to_coeff(config.attack_ms, self.sample_rate);
        self.decay_coeff = Self::time_to_coeff(config.decay_ms, self.sample_rate);
        self.hang_samples = (config.hang_ms * self.sample_rate / 1000.0) as u32;
        self.config = config;
    }

    /// Reset AGC state.
    pub fn reset(&mut self) {
        self.gain = 1.0;
        self.hang_counter = 0;
        self.peak_level = 0.0;
    }
}

/// S-Meter for signal strength display.
///
/// Converts signal level to S-units (S0-S9, S9+10dB, etc.).
#[derive(Clone, Debug)]
pub struct SMeter {
    /// Current signal level in dB (relative)
    level_db: f32,
    /// Smoothing coefficient
    coeff: f32,
    /// Reference level for S9 (in linear units, for future calibration)
    #[allow(dead_code)]
    s9_reference: f32,
}

impl SMeter {
    /// Create a new S-meter.
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz
    /// * `smoothing_ms` - Smoothing time constant in milliseconds
    #[must_use]
    pub fn new(sample_rate: f32, smoothing_ms: f32) -> Self {
        let coeff = if smoothing_ms <= 0.0 {
            1.0
        } else {
            1.0 - (-1.0 / (smoothing_ms * sample_rate / 1000.0)).exp()
        };

        Self {
            level_db: -120.0,
            coeff,
            s9_reference: 0.1, // Calibrate to your system
        }
    }

    /// Update S-meter with a new signal sample (magnitude).
    pub fn update(&mut self, magnitude: f32) {
        let db = if magnitude > 1e-10 {
            20.0 * magnitude.log10()
        } else {
            -120.0
        };

        self.level_db += self.coeff * (db - self.level_db);
    }

    /// Get current level in dB.
    #[must_use]
    pub fn level_db(&self) -> f32 {
        self.level_db
    }

    /// Get S-meter value (0.0 to 1.0 for S0-S9, >1.0 for S9+).
    ///
    /// Each S-unit is 6 dB.
    #[must_use]
    pub fn value(&self) -> f32 {
        // S9 = 0 dB reference, each S-unit = 6 dB
        // Map: S0 = -54 dB (relative to S9), S9 = 0 dB
        let s_units = (self.level_db + 54.0) / 6.0;
        s_units / 9.0 // Normalize so S9 = 1.0
    }

    /// Get S-meter reading as string (e.g., "S7", "S9+10").
    #[must_use]
    pub fn reading(&self) -> SmeterReading {
        let s_units = (self.level_db + 54.0) / 6.0;

        if s_units < 0.0 {
            SmeterReading::S(0)
        } else if s_units <= 9.0 {
            SmeterReading::S(s_units as u8)
        } else {
            let over = ((s_units - 9.0) * 6.0) as i8;
            SmeterReading::S9Plus(over)
        }
    }

    /// Reset S-meter.
    pub fn reset(&mut self) {
        self.level_db = -120.0;
    }
}

impl Default for SMeter {
    fn default() -> Self {
        Self::new(48000.0, 100.0)
    }
}

/// S-meter reading representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmeterReading {
    /// S-units (0-9)
    S(u8),
    /// S9 plus dB over
    S9Plus(i8),
}

impl SmeterReading {
    /// Convert to display string.
    #[must_use]
    pub fn to_string(&self) -> heapless::String<8> {
        let mut s = heapless::String::new();
        match self {
            SmeterReading::S(n) => {
                let _ = core::fmt::write(&mut s, format_args!("S{}", n));
            }
            SmeterReading::S9Plus(db) => {
                let _ = core::fmt::write(&mut s, format_args!("S9+{}", db));
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agc_reduces_loud_signal() {
        let mut agc = Agc::new(48000.0, AgcConfig::default());

        // Process loud signal
        let mut output = 0.0;
        for _ in 0..1000 {
            output = agc.process(1.0);
        }

        // Output should be reduced to near target level
        assert!(output < 0.5);
        assert!(agc.gain() < 1.0);
    }

    #[test]
    fn test_agc_amplifies_quiet_signal() {
        let mut agc = Agc::new(48000.0, AgcConfig::default());

        // Process quiet signal
        for _ in 0..10000 {
            agc.process(0.001);
        }

        // Gain should increase
        assert!(agc.gain() > 1.0);
    }

    #[test]
    fn test_smeter_s_units() {
        let mut meter = SMeter::new(48000.0, 10.0);

        // Update with various levels
        // magnitude 0.1 -> -20 dB -> S-units = (-20+54)/6 = ~5.6
        for _ in 0..1000 {
            meter.update(0.1);
        }

        let reading = meter.reading();
        // Should be around S5-S6
        match reading {
            SmeterReading::S(n) => assert!(n >= 3 && n <= 7, "Expected S3-S7, got S{}", n),
            SmeterReading::S9Plus(_) => panic!("Expected S4-S7, got S9+"),
        }
    }

    #[test]
    fn test_smeter_reading_string() {
        let s5 = SmeterReading::S(5);
        assert_eq!(s5.to_string().as_str(), "S5");

        let s9plus = SmeterReading::S9Plus(20);
        assert_eq!(s9plus.to_string().as_str(), "S9+20");
    }
}
