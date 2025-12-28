//! Digital Filters
//!
//! Provides FIR and IIR filters for audio processing.
//! Uses fixed-point arithmetic for efficient embedded operation.

use fixed::types::I1F15;
#[cfg(feature = "embedded")]
use micromath::F32Ext;

/// Fixed-point sample type (Q1.15 format)
pub type Sample = I1F15;

/// Convert f32 to fixed-point sample
#[must_use]
pub fn to_sample(value: f32) -> Sample {
    Sample::from_num(value.clamp(-1.0, 0.99997))
}

/// Convert fixed-point sample to f32
#[must_use]
pub fn from_sample(sample: Sample) -> f32 {
    sample.to_num::<f32>()
}

/// FIR filter coefficients
#[derive(Clone)]
pub struct FirCoefficients<const N: usize> {
    /// Filter coefficients (symmetric for linear phase)
    taps: [Sample; N],
}

impl<const N: usize> FirCoefficients<N> {
    /// Create coefficients from f32 array
    #[must_use]
    pub fn from_f32(coeffs: &[f32; N]) -> Self {
        let mut taps = [Sample::from_num(0); N];
        for (i, &c) in coeffs.iter().enumerate() {
            taps[i] = to_sample(c);
        }
        Self { taps }
    }

    /// Get coefficient at index
    #[must_use]
    pub fn get(&self, index: usize) -> Sample {
        self.taps.get(index).copied().unwrap_or(Sample::from_num(0))
    }

    /// Generate lowpass filter coefficients using windowed sinc
    #[must_use]
    pub fn lowpass(cutoff_normalized: f32) -> Self {
        let mut coeffs = [0.0f32; N];
        let m = N - 1;
        let fc = cutoff_normalized.clamp(0.0, 0.5);

        for i in 0..N {
            let n = i as f32 - m as f32 / 2.0;
            if n.abs() < 0.0001 {
                coeffs[i] = 2.0 * fc;
            } else {
                coeffs[i] = (2.0 * core::f32::consts::PI * fc * n).sin() / (core::f32::consts::PI * n);
            }

            // Apply Hamming window
            let window = 0.54 - 0.46 * (2.0 * core::f32::consts::PI * i as f32 / m as f32).cos();
            coeffs[i] *= window;
        }

        // Normalize
        let sum: f32 = coeffs.iter().sum();
        if sum.abs() > 0.0001 {
            for c in &mut coeffs {
                *c /= sum;
            }
        }

        Self::from_f32(&coeffs)
    }

    /// Generate bandpass filter coefficients
    #[must_use]
    pub fn bandpass(low_normalized: f32, high_normalized: f32) -> Self {
        let mut coeffs = [0.0f32; N];
        let m = N - 1;
        let fl = low_normalized.clamp(0.0, 0.5);
        let fh = high_normalized.clamp(0.0, 0.5);

        for i in 0..N {
            let n = i as f32 - m as f32 / 2.0;
            if n.abs() < 0.0001 {
                coeffs[i] = 2.0 * (fh - fl);
            } else {
                let pi_n = core::f32::consts::PI * n;
                coeffs[i] = (2.0 * core::f32::consts::PI * fh * n).sin() / pi_n
                    - (2.0 * core::f32::consts::PI * fl * n).sin() / pi_n;
            }

            // Apply Hamming window
            let window = 0.54 - 0.46 * (2.0 * core::f32::consts::PI * i as f32 / m as f32).cos();
            coeffs[i] *= window;
        }

        Self::from_f32(&coeffs)
    }
}

/// FIR filter state
pub struct FirFilter<const N: usize> {
    /// Filter coefficients
    coeffs: FirCoefficients<N>,
    /// Delay line (circular buffer)
    delay: [Sample; N],
    /// Current position in delay line
    pos: usize,
}

impl<const N: usize> FirFilter<N> {
    /// Create a new FIR filter with given coefficients
    #[must_use]
    pub fn new(coeffs: FirCoefficients<N>) -> Self {
        Self {
            coeffs,
            delay: [Sample::from_num(0); N],
            pos: 0,
        }
    }

    /// Process a single sample
    pub fn process(&mut self, input: Sample) -> Sample {
        // Store input in delay line
        self.delay[self.pos] = input;

        // Compute convolution
        let mut acc = Sample::from_num(0);
        let mut idx = self.pos;

        for i in 0..N {
            // Use saturating arithmetic to prevent overflow
            let product = self.delay[idx].saturating_mul(self.coeffs.get(i));
            acc = acc.saturating_add(product);

            if idx == 0 {
                idx = N - 1;
            } else {
                idx -= 1;
            }
        }

        // Advance position
        self.pos = (self.pos + 1) % N;

        acc
    }

    /// Process a block of samples in-place
    pub fn process_block(&mut self, samples: &mut [Sample]) {
        for sample in samples.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.delay.fill(Sample::from_num(0));
        self.pos = 0;
    }

    /// Update coefficients (resets state)
    pub fn set_coefficients(&mut self, coeffs: FirCoefficients<N>) {
        self.coeffs = coeffs;
        self.reset();
    }
}

/// Biquad (second-order IIR) filter coefficients
#[derive(Clone, Copy, Debug)]
pub struct BiquadCoeffs {
    /// Numerator coefficients (b0, b1, b2)
    b: [f32; 3],
    /// Denominator coefficients (a1, a2) - a0 is always 1
    a: [f32; 2],
}

impl BiquadCoeffs {
    /// Create lowpass biquad filter
    #[must_use]
    pub fn lowpass(freq_normalized: f32, q: f32) -> Self {
        let w0 = 2.0 * core::f32::consts::PI * freq_normalized;
        let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b: [b0 / a0, b1 / a0, b2 / a0],
            a: [a1 / a0, a2 / a0],
        }
    }

    /// Create highpass biquad filter
    #[must_use]
    pub fn highpass(freq_normalized: f32, q: f32) -> Self {
        let w0 = 2.0 * core::f32::consts::PI * freq_normalized;
        let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
        let alpha = sin_w0 / (2.0 * q);

        let b0 = f32::midpoint(1.0, cos_w0);
        let b1 = -(1.0 + cos_w0);
        let b2 = f32::midpoint(1.0, cos_w0);
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b: [b0 / a0, b1 / a0, b2 / a0],
            a: [a1 / a0, a2 / a0],
        }
    }

    /// Create bandpass biquad filter
    #[must_use]
    pub fn bandpass(freq_normalized: f32, q: f32) -> Self {
        let w0 = 2.0 * core::f32::consts::PI * freq_normalized;
        let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
        let alpha = sin_w0 / (2.0 * q);

        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b: [b0 / a0, b1 / a0, b2 / a0],
            a: [a1 / a0, a2 / a0],
        }
    }

    /// Create notch filter
    #[must_use]
    pub fn notch(freq_normalized: f32, q: f32) -> Self {
        let w0 = 2.0 * core::f32::consts::PI * freq_normalized;
        let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
        let alpha = sin_w0 / (2.0 * q);

        let b0 = 1.0;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b: [b0 / a0, b1 / a0, b2 / a0],
            a: [a1 / a0, a2 / a0],
        }
    }
}

/// Biquad filter state
#[derive(Clone, Copy, Debug, Default)]
pub struct BiquadFilter {
    coeffs: Option<BiquadCoeffs>,
    /// State variables (Direct Form II Transposed)
    z: [f32; 2],
}

impl BiquadFilter {
    /// Create a new biquad filter
    #[must_use]
    pub const fn new() -> Self {
        Self {
            coeffs: None,
            z: [0.0; 2],
        }
    }

    /// Create with coefficients
    #[must_use]
    pub fn with_coeffs(coeffs: BiquadCoeffs) -> Self {
        Self {
            coeffs: Some(coeffs),
            z: [0.0; 2],
        }
    }

    /// Set coefficients
    pub fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = Some(coeffs);
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        let Some(c) = &self.coeffs else {
            return input;
        };

        let output = c.b[0] * input + self.z[0];
        self.z[0] = c.b[1] * input - c.a[0] * output + self.z[1];
        self.z[1] = c.b[2] * input - c.a[1] * output;

        output
    }

    /// Process a block of samples in-place
    pub fn process_block(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.z = [0.0; 2];
    }
}

/// DC blocking filter (simple IIR highpass)
#[derive(Clone, Copy, Debug)]
pub struct DcBlocker {
    /// Previous input
    x_prev: f32,
    /// Previous output
    y_prev: f32,
    /// Filter coefficient (0.99 typical)
    alpha: f32,
}

impl DcBlocker {
    /// Create a new DC blocker
    #[must_use]
    pub const fn new(alpha: f32) -> Self {
        Self {
            x_prev: 0.0,
            y_prev: 0.0,
            alpha,
        }
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        let output = input - self.x_prev + self.alpha * self.y_prev;
        self.x_prev = input;
        self.y_prev = output;
        output
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.x_prev = 0.0;
        self.y_prev = 0.0;
    }
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self::new(0.995)
    }
}

/// Moving average filter for smoothing
#[derive(Clone)]
pub struct MovingAverage<const N: usize> {
    buffer: [f32; N],
    sum: f32,
    pos: usize,
}

impl<const N: usize> MovingAverage<N> {
    /// Create a new moving average filter
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0.0; N],
            sum: 0.0,
            pos: 0,
        }
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        self.sum -= self.buffer[self.pos];
        self.sum += input;
        self.buffer[self.pos] = input;
        self.pos = (self.pos + 1) % N;
        self.sum / N as f32
    }

    /// Get current average
    #[must_use]
    pub fn average(&self) -> f32 {
        self.sum / N as f32
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.sum = 0.0;
        self.pos = 0;
    }
}

impl<const N: usize> Default for MovingAverage<N> {
    fn default() -> Self {
        Self::new()
    }
}
