//! Numerically Controlled Oscillator (NCO) implementations.
//!
//! Provides digital oscillators for frequency mixing, carrier generation,
//! and quadrature signal generation.

use crate::types::IqSample;
#[allow(unused_imports)]
use micromath::F32Ext;

/// Numerically Controlled Oscillator.
///
/// Generates sine and cosine outputs at a programmable frequency
/// using a phase accumulator.
#[derive(Clone, Debug)]
pub struct Nco {
    /// Current phase in radians (0 to 2π)
    phase: f32,
    /// Phase increment per sample (frequency)
    phase_inc: f32,
    /// Sample rate in Hz
    sample_rate: f32,
}

impl Nco {
    /// Create a new NCO.
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz
    /// * `frequency` - Initial frequency in Hz
    #[must_use]
    pub fn new(sample_rate: f32, frequency: f32) -> Self {
        let phase_inc = 2.0 * core::f32::consts::PI * frequency / sample_rate;
        Self {
            phase: 0.0,
            phase_inc,
            sample_rate,
        }
    }

    /// Set oscillator frequency in Hz.
    pub fn set_frequency(&mut self, frequency: f32) {
        self.phase_inc = 2.0 * core::f32::consts::PI * frequency / self.sample_rate;
    }

    /// Adjust frequency by delta Hz (for AFC).
    pub fn adjust_frequency(&mut self, delta_hz: f32) {
        let delta_inc = 2.0 * core::f32::consts::PI * delta_hz / self.sample_rate;
        self.phase_inc += delta_inc;
    }

    /// Get current frequency in Hz.
    #[must_use]
    pub fn frequency(&self) -> f32 {
        self.phase_inc * self.sample_rate / (2.0 * core::f32::consts::PI)
    }

    /// Get current phase in radians.
    #[must_use]
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Set phase in radians.
    pub fn set_phase(&mut self, phase: f32) {
        self.phase = wrap_phase(phase);
    }

    /// Adjust phase by delta radians.
    pub fn adjust_phase(&mut self, delta: f32) {
        self.phase = wrap_phase(self.phase + delta);
    }

    /// Generate next sine sample and advance phase.
    #[inline]
    pub fn next_sin(&mut self) -> f32 {
        let output = self.phase.sin();
        self.advance();
        output
    }

    /// Generate next cosine sample and advance phase.
    #[inline]
    pub fn next_cos(&mut self) -> f32 {
        let output = self.phase.cos();
        self.advance();
        output
    }

    /// Generate next IQ sample (cos + j*sin) and advance phase.
    ///
    /// Returns complex exponential e^(j*phase).
    #[inline]
    pub fn next_iq(&mut self) -> IqSample {
        let (sin, cos) = (self.phase.sin(), self.phase.cos());
        self.advance();
        IqSample::new(cos, sin)
    }

    /// Advance phase without generating output.
    #[inline]
    pub fn advance(&mut self) {
        self.phase = wrap_phase(self.phase + self.phase_inc);
    }

    /// Reset phase to zero.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Mix (multiply) an IQ sample with the NCO output.
    ///
    /// This shifts the signal by the NCO frequency.
    #[inline]
    pub fn mix(&mut self, input: IqSample) -> IqSample {
        let lo = self.next_iq();
        input.multiply(lo.conjugate())
    }
}

/// Wrap phase to range [-π, π].
#[inline]
fn wrap_phase(phase: f32) -> f32 {
    let mut p = phase;
    while p > core::f32::consts::PI {
        p -= 2.0 * core::f32::consts::PI;
    }
    while p < -core::f32::consts::PI {
        p += 2.0 * core::f32::consts::PI;
    }
    p
}

/// Quadrature oscillator generating I and Q outputs in quadrature.
///
/// Optimized version using coupled form for reduced computation.
#[derive(Clone, Debug)]
pub struct QuadratureOscillator {
    /// Cosine state (I)
    cos_state: f32,
    /// Sine state (Q)
    sin_state: f32,
    /// Update coefficients
    alpha: f32,
    beta: f32,
    /// Sample rate
    sample_rate: f32,
}

impl QuadratureOscillator {
    /// Create a new quadrature oscillator.
    #[must_use]
    pub fn new(sample_rate: f32, frequency: f32) -> Self {
        let omega = 2.0 * core::f32::consts::PI * frequency / sample_rate;
        Self {
            cos_state: 1.0,
            sin_state: 0.0,
            alpha: omega.sin(),
            beta: omega.cos(),
            sample_rate,
        }
    }

    /// Set frequency in Hz.
    pub fn set_frequency(&mut self, frequency: f32) {
        let omega = 2.0 * core::f32::consts::PI * frequency / self.sample_rate;
        self.alpha = omega.sin();
        self.beta = omega.cos();
    }

    /// Generate next IQ sample.
    #[inline]
    pub fn next(&mut self) -> IqSample {
        let output = IqSample::new(self.cos_state, self.sin_state);

        // Coupled form oscillator update
        let new_cos = self.beta * self.cos_state - self.alpha * self.sin_state;
        let new_sin = self.alpha * self.cos_state + self.beta * self.sin_state;

        // Normalize to prevent amplitude drift
        let mag = (new_cos * new_cos + new_sin * new_sin).sqrt();
        if mag > 0.0 {
            self.cos_state = new_cos / mag;
            self.sin_state = new_sin / mag;
        }

        output
    }

    /// Reset to initial state.
    pub fn reset(&mut self) {
        self.cos_state = 1.0;
        self.sin_state = 0.0;
    }
}

/// Costas loop for carrier tracking.
///
/// Used for BPSK/QPSK demodulation to track carrier phase and frequency.
#[derive(Clone, Debug)]
pub struct CostasLoop {
    /// NCO for carrier generation
    nco: Nco,
    /// Loop filter integrator
    integrator: f32,
    /// Proportional gain (phase tracking)
    kp: f32,
    /// Integral gain (frequency tracking)
    ki: f32,
    /// Maximum frequency offset in Hz
    max_freq_offset: f32,
}

impl CostasLoop {
    /// Create a new Costas loop.
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz
    /// * `center_freq` - Expected carrier frequency in Hz
    /// * `loop_bandwidth` - Loop bandwidth in Hz (affects tracking speed)
    #[must_use]
    pub fn new(sample_rate: f32, center_freq: f32, loop_bandwidth: f32) -> Self {
        // Calculate loop gains from bandwidth using standard formulas
        let damping = 0.707; // Butterworth response
        let bn_ts = loop_bandwidth / sample_rate;
        let denom = 1.0 + 2.0 * damping * bn_ts + bn_ts * bn_ts;

        let kp = 4.0 * damping * bn_ts / denom;
        let ki = 4.0 * bn_ts * bn_ts / denom;

        Self {
            nco: Nco::new(sample_rate, center_freq),
            integrator: 0.0,
            kp,
            ki,
            max_freq_offset: loop_bandwidth * 2.0,
        }
    }

    /// Process an IQ sample and return tracked output with phase error.
    ///
    /// Returns (tracked_sample, phase_error).
    pub fn process(&mut self, input: IqSample) -> (IqSample, f32) {
        // Mix down with NCO
        let mixed = self.nco.mix(input);

        // Calculate phase error (for BPSK: I * sign(Q))
        let phase_error = mixed.i * mixed.q.signum();

        // Update loop filter
        self.integrator += self.ki * phase_error;

        // Clamp frequency offset
        self.integrator = self
            .integrator
            .clamp(-self.max_freq_offset, self.max_freq_offset);

        // Update NCO frequency
        let freq_correction = self.kp * phase_error + self.integrator;
        self.nco.adjust_frequency(freq_correction);

        (mixed, phase_error)
    }

    /// Get current frequency offset in Hz.
    #[must_use]
    pub fn frequency_offset(&self) -> f32 {
        self.integrator
    }

    /// Reset loop state.
    pub fn reset(&mut self) {
        self.nco.reset();
        self.integrator = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nco_frequency() {
        let mut nco = Nco::new(48000.0, 1000.0);
        assert!((nco.frequency() - 1000.0).abs() < 1e-3);

        nco.set_frequency(2000.0);
        assert!((nco.frequency() - 2000.0).abs() < 1e-3);
    }

    #[test]
    fn test_nco_generates_sine() {
        let mut nco = Nco::new(48000.0, 1000.0);
        let mut prev = -0.1f32; // Start negative to detect first zero crossing
        let mut zero_crossings = 0;

        // Count zero crossings in 96 samples (2ms at 48kHz = 2 cycles)
        for _ in 0..96 {
            let sample = nco.next_sin();
            if prev < 0.0 && sample >= 0.0 {
                zero_crossings += 1;
            }
            prev = sample;
        }

        // 1000 Hz should have ~2 complete cycles per 2ms
        assert!(zero_crossings >= 1 && zero_crossings <= 3, "Expected 1-3 zero crossings, got {}", zero_crossings);
    }

    #[test]
    fn test_nco_iq_quadrature() {
        let mut nco = Nco::new(48000.0, 1000.0);

        // I and Q should be 90 degrees apart
        let iq = nco.next_iq();

        // For phase=0: I=cos(0)=1, Q=sin(0)=0
        assert!((iq.i - 1.0).abs() < 1e-6);
        assert!(iq.q.abs() < 1e-6);
    }

    #[test]
    fn test_quadrature_oscillator_stable() {
        let mut osc = QuadratureOscillator::new(48000.0, 1000.0);

        // Run for many samples and check amplitude stability
        for _ in 0..100000 {
            let _ = osc.next();
        }

        let iq = osc.next();
        let mag = iq.magnitude();
        assert!((mag - 1.0).abs() < 0.01, "Amplitude drift: {}", mag);
    }

    #[test]
    fn test_wrap_phase() {
        assert!((wrap_phase(0.0) - 0.0).abs() < 1e-6);
        assert!((wrap_phase(core::f32::consts::PI) - core::f32::consts::PI).abs() < 1e-6);
        // 3π wraps to π (which equals -π in terms of sin/cos)
        let wrapped = wrap_phase(3.0 * core::f32::consts::PI);
        assert!(wrapped.abs() - core::f32::consts::PI < 1e-5);
    }
}
