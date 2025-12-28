//! Digital Oscillators
//!
//! Provides oscillators for tone generation and carrier synthesis.
//! Uses efficient algorithms suitable for embedded systems.

use core::f32::consts::PI;
#[cfg(feature = "embedded")]
use micromath::F32Ext;

/// Sine wave oscillator using direct computation
#[derive(Clone, Copy, Debug)]
pub struct SineOscillator {
    /// Current phase (0.0 to 1.0)
    phase: f32,
    /// Phase increment per sample
    phase_inc: f32,
}

impl SineOscillator {
    /// Create a new sine oscillator
    #[must_use]
    pub const fn new() -> Self {
        Self {
            phase: 0.0,
            phase_inc: 0.0,
        }
    }

    /// Set frequency
    pub fn set_frequency(&mut self, freq_hz: f32, sample_rate: f32) {
        self.phase_inc = freq_hz / sample_rate;
    }

    /// Generate next sample
    pub fn next(&mut self) -> f32 {
        let sample = (self.phase * 2.0 * PI).sin();
        self.phase += self.phase_inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        sample
    }

    /// Generate next sample with phase offset (for quadrature)
    pub fn next_with_offset(&mut self, offset: f32) -> f32 {
        let sample = ((self.phase + offset) * 2.0 * PI).sin();
        self.phase += self.phase_inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        sample
    }

    /// Reset phase
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Get current phase
    #[must_use]
    pub fn phase(&self) -> f32 {
        self.phase
    }
}

impl Default for SineOscillator {
    fn default() -> Self {
        Self::new()
    }
}

/// Quadrature oscillator (I/Q generation)
///
/// Generates sine and cosine simultaneously for efficient
/// quadrature signal processing.
#[derive(Clone, Copy, Debug)]
pub struct QuadratureOscillator {
    /// Sine state (imaginary part)
    sin_state: f32,
    /// Cosine state (real part)
    cos_state: f32,
    /// Rotation coefficient (sine of phase increment)
    sin_inc: f32,
    /// Rotation coefficient (cosine of phase increment)
    cos_inc: f32,
}

impl QuadratureOscillator {
    /// Create a new quadrature oscillator
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sin_state: 0.0,
            cos_state: 1.0,
            sin_inc: 0.0,
            cos_inc: 1.0,
        }
    }

    /// Set frequency
    pub fn set_frequency(&mut self, freq_hz: f32, sample_rate: f32) {
        let phase_inc = 2.0 * PI * freq_hz / sample_rate;
        self.sin_inc = phase_inc.sin();
        self.cos_inc = phase_inc.cos();
    }

    /// Generate next I/Q sample pair
    pub fn next(&mut self) -> (f32, f32) {
        let i = self.cos_state;
        let q = self.sin_state;

        // Complex rotation: (cos + j*sin) * (cos_state + j*sin_state)
        let new_cos = self.cos_state * self.cos_inc - self.sin_state * self.sin_inc;
        let new_sin = self.sin_state * self.cos_inc + self.cos_state * self.sin_inc;

        self.cos_state = new_cos;
        self.sin_state = new_sin;

        // Periodic normalization to prevent drift
        let mag_sq = new_cos * new_cos + new_sin * new_sin;
        if (mag_sq - 1.0).abs() > 0.0001 {
            let mag = mag_sq.sqrt();
            self.cos_state /= mag;
            self.sin_state /= mag;
        }

        (i, q)
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.sin_state = 0.0;
        self.cos_state = 1.0;
    }
}

impl Default for QuadratureOscillator {
    fn default() -> Self {
        Self::new()
    }
}

/// NCO (Numerically Controlled Oscillator) with phase accumulator
///
/// Uses a phase accumulator with lookup table for efficient
/// sine generation. Good for fixed-point implementations.
#[derive(Clone, Copy, Debug)]
pub struct Nco {
    /// Phase accumulator (32-bit for precision)
    phase: u32,
    /// Phase increment per sample
    phase_inc: u32,
}

impl Nco {
    /// Create a new NCO
    #[must_use]
    pub const fn new() -> Self {
        Self {
            phase: 0,
            phase_inc: 0,
        }
    }

    /// Set frequency (integer Hz at given sample rate)
    pub fn set_frequency(&mut self, freq_hz: u32, sample_rate: u32) {
        // phase_inc = freq * 2^32 / sample_rate
        self.phase_inc = ((u64::from(freq_hz) * (1u64 << 32)) / u64::from(sample_rate)) as u32;
    }

    /// Set frequency with fractional Hz
    pub fn set_frequency_f32(&mut self, freq_hz: f32, sample_rate: f32) {
        self.phase_inc = (freq_hz / sample_rate * 4294967296.0) as u32;
    }

    /// Get next phase value (0 to 2^32-1)
    pub fn next_phase(&mut self) -> u32 {
        let current = self.phase;
        self.phase = self.phase.wrapping_add(self.phase_inc);
        current
    }

    /// Get next sample using sine lookup
    pub fn next(&mut self) -> f32 {
        let phase = self.next_phase();
        // Convert phase to radians and compute sine
        let radians = (phase as f32 / 4294967296.0) * 2.0 * PI;
        radians.sin()
    }

    /// Get next I/Q pair
    pub fn next_iq(&mut self) -> (f32, f32) {
        let phase = self.next_phase();
        let radians = (phase as f32 / 4294967296.0) * 2.0 * PI;
        (radians.cos(), radians.sin())
    }

    /// Reset phase
    pub fn reset(&mut self) {
        self.phase = 0;
    }

    /// Set phase directly
    pub fn set_phase(&mut self, phase: u32) {
        self.phase = phase;
    }
}

impl Default for Nco {
    fn default() -> Self {
        Self::new()
    }
}

/// CW sidetone oscillator with envelope shaping
#[derive(Clone, Copy, Debug)]
pub struct CwToneGenerator {
    /// Tone oscillator
    osc: SineOscillator,
    /// Current envelope level
    envelope: f32,
    /// Envelope attack rate
    attack_rate: f32,
    /// Envelope decay rate
    decay_rate: f32,
    /// Key state
    key_down: bool,
}

impl CwToneGenerator {
    /// Create a new CW tone generator
    #[must_use]
    pub fn new(freq_hz: f32, sample_rate: f32) -> Self {
        let mut osc = SineOscillator::new();
        osc.set_frequency(freq_hz, sample_rate);

        // 5ms attack/decay at sample rate
        let rate = 1.0 / (0.005 * sample_rate);

        Self {
            osc,
            envelope: 0.0,
            attack_rate: rate,
            decay_rate: rate,
            key_down: false,
        }
    }

    /// Set key state
    pub fn set_key(&mut self, down: bool) {
        self.key_down = down;
    }

    /// Set tone frequency
    pub fn set_frequency(&mut self, freq_hz: f32, sample_rate: f32) {
        self.osc.set_frequency(freq_hz, sample_rate);
    }

    /// Set rise/fall time in milliseconds
    pub fn set_rise_time(&mut self, ms: f32, sample_rate: f32) {
        let rate = 1.0 / (ms / 1000.0 * sample_rate);
        self.attack_rate = rate;
        self.decay_rate = rate;
    }

    /// Generate next sample
    pub fn next(&mut self) -> f32 {
        // Update envelope
        if self.key_down {
            self.envelope = (self.envelope + self.attack_rate).min(1.0);
        } else {
            self.envelope = (self.envelope - self.decay_rate).max(0.0);
        }

        // Generate shaped tone
        if self.envelope > 0.0001 {
            self.osc.next() * self.envelope
        } else {
            0.0
        }
    }

    /// Check if tone is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.envelope > 0.0001 || self.key_down
    }
}

/// DTMF tone generator
#[derive(Clone, Copy, Debug)]
pub struct DtmfGenerator {
    /// Low frequency oscillator
    low_osc: SineOscillator,
    /// High frequency oscillator
    high_osc: SineOscillator,
    /// Envelope for soft keying
    envelope: f32,
    /// Attack/decay rate
    rate: f32,
    /// Active state
    active: bool,
}

impl DtmfGenerator {
    /// DTMF low group frequencies
    const LOW_FREQS: [f32; 4] = [697.0, 770.0, 852.0, 941.0];
    /// DTMF high group frequencies
    const HIGH_FREQS: [f32; 4] = [1209.0, 1336.0, 1477.0, 1633.0];

    /// Create a new DTMF generator
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let mut gen = Self {
            low_osc: SineOscillator::new(),
            high_osc: SineOscillator::new(),
            envelope: 0.0,
            rate: 1.0 / (0.002 * sample_rate), // 2ms rise time
            active: false,
        };
        gen.low_osc.set_frequency(697.0, sample_rate);
        gen.high_osc.set_frequency(1209.0, sample_rate);
        gen
    }

    /// Set digit (0-9, *, #, A-D)
    pub fn set_digit(&mut self, digit: char, sample_rate: f32) {
        let (low_idx, high_idx) = match digit {
            '1' => (0, 0),
            '2' => (0, 1),
            '3' => (0, 2),
            'A' => (0, 3),
            '4' => (1, 0),
            '5' => (1, 1),
            '6' => (1, 2),
            'B' => (1, 3),
            '7' => (2, 0),
            '8' => (2, 1),
            '9' => (2, 2),
            'C' => (2, 3),
            '*' => (3, 0),
            '0' => (3, 1),
            '#' => (3, 2),
            'D' => (3, 3),
            _ => return,
        };

        self.low_osc.set_frequency(Self::LOW_FREQS[low_idx], sample_rate);
        self.high_osc.set_frequency(Self::HIGH_FREQS[high_idx], sample_rate);
        self.active = true;
    }

    /// Stop tone
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Generate next sample
    pub fn next(&mut self) -> f32 {
        // Update envelope
        if self.active {
            self.envelope = (self.envelope + self.rate).min(1.0);
        } else {
            self.envelope = (self.envelope - self.rate).max(0.0);
        }

        if self.envelope > 0.0001 {
            (self.low_osc.next() + self.high_osc.next()) * 0.5 * self.envelope
        } else {
            0.0
        }
    }

    /// Check if tone is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.envelope > 0.0001 || self.active
    }
}
