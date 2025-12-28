//! Digital filter implementations.
//!
//! Provides biquad IIR filters, FIR filters, and DC blocking filters
//! for audio and signal processing.

use crate::types::IqSample;
#[allow(unused_imports)]
use micromath::F32Ext;

/// Biquad filter coefficients.
///
/// Implements the standard Direct Form II biquad:
/// ```text
/// y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
/// ```
#[derive(Clone, Copy, Debug)]
pub struct BiquadCoeffs {
    /// Feedforward coefficient b0
    pub b0: f32,
    /// Feedforward coefficient b1
    pub b1: f32,
    /// Feedforward coefficient b2
    pub b2: f32,
    /// Feedback coefficient a1 (note: negated in difference equation)
    pub a1: f32,
    /// Feedback coefficient a2 (note: negated in difference equation)
    pub a2: f32,
}

impl BiquadCoeffs {
    /// Create unity (pass-through) filter coefficients.
    #[must_use]
    pub const fn unity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Design a lowpass filter.
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz
    /// * `cutoff` - Cutoff frequency in Hz
    /// * `q` - Quality factor (0.707 for Butterworth)
    #[must_use]
    pub fn lowpass(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        let omega = 2.0 * core::f32::consts::PI * cutoff / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = (1.0 - cos_omega) / 2.0;
        let b1 = 1.0 - cos_omega;
        let b2 = (1.0 - cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Design a highpass filter.
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz
    /// * `cutoff` - Cutoff frequency in Hz
    /// * `q` - Quality factor (0.707 for Butterworth)
    #[must_use]
    pub fn highpass(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        let omega = 2.0 * core::f32::consts::PI * cutoff / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = (1.0 + cos_omega) / 2.0;
        let b1 = -(1.0 + cos_omega);
        let b2 = (1.0 + cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Design a bandpass filter (constant 0 dB peak gain).
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz
    /// * `center` - Center frequency in Hz
    /// * `bandwidth` - Bandwidth in Hz
    #[must_use]
    pub fn bandpass(sample_rate: f32, center: f32, bandwidth: f32) -> Self {
        let omega = 2.0 * core::f32::consts::PI * center / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let q = center / bandwidth;
        let alpha = sin_omega / (2.0 * q);

        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Design a notch (band-reject) filter.
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz
    /// * `center` - Notch center frequency in Hz
    /// * `bandwidth` - Bandwidth in Hz
    #[must_use]
    pub fn notch(sample_rate: f32, center: f32, bandwidth: f32) -> Self {
        let omega = 2.0 * core::f32::consts::PI * center / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let q = center / bandwidth;
        let alpha = sin_omega / (2.0 * q);

        let b0 = 1.0;
        let b1 = -2.0 * cos_omega;
        let b2 = 1.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        Self::unity()
    }
}

/// Biquad filter state (Direct Form II Transposed).
#[derive(Clone, Debug, Default)]
pub struct Biquad {
    coeffs: BiquadCoeffs,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// Create a new biquad filter with given coefficients.
    #[must_use]
    pub const fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            coeffs,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// Create a lowpass biquad filter.
    #[must_use]
    pub fn lowpass(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        Self::new(BiquadCoeffs::lowpass(sample_rate, cutoff, q))
    }

    /// Create a highpass biquad filter.
    #[must_use]
    pub fn highpass(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        Self::new(BiquadCoeffs::highpass(sample_rate, cutoff, q))
    }

    /// Create a bandpass biquad filter.
    #[must_use]
    pub fn bandpass(sample_rate: f32, center: f32, bandwidth: f32) -> Self {
        Self::new(BiquadCoeffs::bandpass(sample_rate, center, bandwidth))
    }

    /// Create a notch biquad filter.
    #[must_use]
    pub fn notch(sample_rate: f32, center: f32, bandwidth: f32) -> Self {
        Self::new(BiquadCoeffs::notch(sample_rate, center, bandwidth))
    }

    /// Process a single sample.
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.coeffs.b0 * input + self.z1;
        self.z1 = self.coeffs.b1 * input - self.coeffs.a1 * output + self.z2;
        self.z2 = self.coeffs.b2 * input - self.coeffs.a2 * output;
        output
    }

    /// Process an IQ sample (applies filter to both I and Q).
    #[inline]
    pub fn process_iq(&mut self, input: IqSample) -> IqSample {
        // Note: This applies the same filter to both channels.
        // For proper IQ filtering, use two separate biquads.
        IqSample::new(self.process(input.i), self.process(input.q))
    }

    /// Reset filter state to zero.
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    /// Update filter coefficients (keeps state for smooth transition).
    pub fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }
}

/// Biquad filter pair for IQ processing.
///
/// Uses separate filter instances for I and Q channels.
#[derive(Clone, Debug)]
pub struct BiquadIq {
    i_filter: Biquad,
    q_filter: Biquad,
}

impl BiquadIq {
    /// Create a new IQ biquad filter pair.
    #[must_use]
    pub fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            i_filter: Biquad::new(coeffs),
            q_filter: Biquad::new(coeffs),
        }
    }

    /// Create a lowpass IQ filter.
    #[must_use]
    pub fn lowpass(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        Self::new(BiquadCoeffs::lowpass(sample_rate, cutoff, q))
    }

    /// Process an IQ sample.
    #[inline]
    pub fn process(&mut self, input: IqSample) -> IqSample {
        IqSample::new(self.i_filter.process(input.i), self.q_filter.process(input.q))
    }

    /// Reset both filter states.
    pub fn reset(&mut self) {
        self.i_filter.reset();
        self.q_filter.reset();
    }
}

/// DC blocking filter.
///
/// Removes DC offset from a signal using a simple IIR highpass.
/// Transfer function: H(z) = (1 - z^-1) / (1 - alpha * z^-1)
#[derive(Clone, Debug)]
pub struct DcBlocker {
    alpha: f32,
    prev_input: f32,
    prev_output: f32,
}

impl DcBlocker {
    /// Create a new DC blocker.
    ///
    /// # Arguments
    /// * `alpha` - Feedback coefficient (0.99 to 0.999 typical, higher = lower cutoff)
    #[must_use]
    pub const fn new(alpha: f32) -> Self {
        Self {
            alpha,
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }

    /// Create a DC blocker with default alpha (0.995).
    #[must_use]
    pub const fn default_alpha() -> Self {
        Self::new(0.995)
    }

    /// Process a single sample.
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let output = input - self.prev_input + self.alpha * self.prev_output;
        self.prev_input = input;
        self.prev_output = output;
        output
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.prev_input = 0.0;
        self.prev_output = 0.0;
    }
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self::default_alpha()
    }
}

/// DC blocker for IQ signals.
#[derive(Clone, Debug, Default)]
pub struct DcBlockerIq {
    i_blocker: DcBlocker,
    q_blocker: DcBlocker,
}

impl DcBlockerIq {
    /// Create a new IQ DC blocker.
    #[must_use]
    pub const fn new(alpha: f32) -> Self {
        Self {
            i_blocker: DcBlocker::new(alpha),
            q_blocker: DcBlocker::new(alpha),
        }
    }

    /// Process an IQ sample.
    #[inline]
    pub fn process(&mut self, input: IqSample) -> IqSample {
        IqSample::new(
            self.i_blocker.process(input.i),
            self.q_blocker.process(input.q),
        )
    }

    /// Reset both blockers.
    pub fn reset(&mut self) {
        self.i_blocker.reset();
        self.q_blocker.reset();
    }
}

/// Simple FIR filter with fixed tap count.
///
/// Uses a circular buffer for efficient processing.
#[derive(Clone, Debug)]
pub struct FirFilter<const N: usize> {
    coeffs: [f32; N],
    buffer: [f32; N],
    write_pos: usize,
}

impl<const N: usize> FirFilter<N> {
    /// Create a new FIR filter with given coefficients.
    #[must_use]
    pub const fn new(coeffs: [f32; N]) -> Self {
        Self {
            coeffs,
            buffer: [0.0; N],
            write_pos: 0,
        }
    }

    /// Process a single sample.
    pub fn process(&mut self, input: f32) -> f32 {
        // Write new sample to buffer
        self.buffer[self.write_pos] = input;

        // Compute convolution
        let mut output = 0.0;
        let mut read_pos = self.write_pos;

        for coeff in &self.coeffs {
            output += self.buffer[read_pos] * coeff;
            if read_pos == 0 {
                read_pos = N - 1;
            } else {
                read_pos -= 1;
            }
        }

        // Advance write position
        self.write_pos = (self.write_pos + 1) % N;

        output
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.buffer = [0.0; N];
        self.write_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_unity() {
        let mut filter = Biquad::new(BiquadCoeffs::unity());
        assert!((filter.process(1.0) - 1.0).abs() < 1e-6);
        assert!((filter.process(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_dc_blocker_removes_dc() {
        let mut blocker = DcBlocker::new(0.99);

        // Feed constant DC
        for _ in 0..1000 {
            blocker.process(1.0);
        }

        // Output should be near zero after settling
        let output = blocker.process(1.0);
        assert!(output.abs() < 0.1);
    }

    #[test]
    fn test_lowpass_attenuates_high_freq() {
        let mut filter = Biquad::lowpass(48000.0, 1000.0, 0.707);

        // Process a few samples of high frequency signal
        let mut max_output = 0.0f32;
        for i in 0..1000 {
            // 10kHz signal (well above cutoff)
            let input = (i as f32 * 2.0 * core::f32::consts::PI * 10000.0 / 48000.0).sin();
            let output = filter.process(input);
            max_output = max_output.max(output.abs());
        }

        // Should be significantly attenuated
        assert!(max_output < 0.5);
    }
}
