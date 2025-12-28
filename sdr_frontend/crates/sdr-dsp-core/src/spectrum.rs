//! Spectrum analysis and waterfall display.
//!
//! Provides sliding DFT for efficient spectrum computation
//! and data structures for waterfall display.

use micromath::F32Ext;

/// Maximum number of FFT bins supported.
pub const MAX_BINS: usize = 512;

/// Spectrum analyzer configuration.
#[derive(Clone, Copy, Debug)]
pub struct SpectrumConfig {
    /// Number of FFT bins
    pub fft_size: usize,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Averaging count (1 = no averaging)
    pub averaging: u8,
    /// Reference level in dB
    pub ref_level_db: f32,
    /// Display range in dB
    pub range_db: f32,
}

impl Default for SpectrumConfig {
    fn default() -> Self {
        Self {
            fft_size: 256,
            sample_rate: 48000,
            averaging: 4,
            ref_level_db: 0.0,
            range_db: 80.0,
        }
    }
}

impl SpectrumConfig {
    /// Calculate frequency resolution (Hz per bin).
    #[must_use]
    pub fn bin_width(&self) -> f32 {
        self.sample_rate as f32 / self.fft_size as f32
    }

    /// Calculate frequency for a given bin index.
    #[must_use]
    pub fn bin_frequency(&self, bin: usize) -> f32 {
        bin as f32 * self.bin_width()
    }
}

/// Single spectrum bin data.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpectrumBin {
    /// Power level in dB (relative to full scale)
    pub power_db: f32,
}

/// Sliding DFT spectrum analyzer.
///
/// Efficiently computes a subset of DFT bins using the sliding DFT algorithm.
/// More efficient than full FFT when only displaying a portion of the spectrum.
#[derive(Clone)]
pub struct SlidingDft {
    /// Number of bins to compute
    num_bins: usize,
    /// Window size
    window_size: usize,
    /// Circular sample buffer
    buffer: [f32; MAX_BINS],
    /// Current write position
    write_pos: usize,
    /// Accumulated real parts per bin
    real: [f32; MAX_BINS],
    /// Accumulated imaginary parts per bin
    imag: [f32; MAX_BINS],
    /// Power accumulator for averaging
    power_accum: [f32; MAX_BINS],
    /// Sample count for averaging
    sample_count: u32,
    /// Averaging count
    averaging: u8,
}

impl SlidingDft {
    /// Create a new sliding DFT analyzer.
    ///
    /// # Arguments
    /// * `num_bins` - Number of frequency bins (max 512)
    /// * `averaging` - Number of frames to average
    #[must_use]
    pub fn new(num_bins: usize, averaging: u8) -> Self {
        let num_bins = num_bins.min(MAX_BINS);
        Self {
            num_bins,
            window_size: num_bins,
            buffer: [0.0; MAX_BINS],
            write_pos: 0,
            real: [0.0; MAX_BINS],
            imag: [0.0; MAX_BINS],
            power_accum: [0.0; MAX_BINS],
            sample_count: 0,
            averaging,
        }
    }

    /// Get number of bins.
    #[must_use]
    pub fn num_bins(&self) -> usize {
        self.num_bins
    }

    /// Push a new sample and update DFT.
    pub fn push(&mut self, sample: f32) {
        // Remove oldest sample contribution and add new
        let oldest = self.buffer[self.write_pos];
        self.buffer[self.write_pos] = sample;

        // Update DFT bins using Goertzel-like update
        for k in 0..self.num_bins {
            let omega = 2.0 * core::f32::consts::PI * k as f32 / self.window_size as f32;
            let cos_omega = omega.cos();
            let sin_omega = omega.sin();

            // Update using difference (new - old) rotated
            let diff = sample - oldest;
            self.real[k] = self.real[k] * cos_omega - self.imag[k] * sin_omega + diff;
            self.imag[k] = self.real[k] * sin_omega + self.imag[k] * cos_omega;
        }

        // Advance write position
        self.write_pos = (self.write_pos + 1) % self.window_size;
        self.sample_count += 1;
    }

    /// Compute power spectrum and accumulate for averaging.
    pub fn compute(&mut self) {
        for k in 0..self.num_bins {
            let power = self.real[k] * self.real[k] + self.imag[k] * self.imag[k];
            self.power_accum[k] += power;
        }
    }

    /// Get averaged power in dB for a bin.
    #[must_use]
    pub fn power_db(&self, bin: usize) -> f32 {
        if bin >= self.num_bins || self.averaging == 0 {
            return -120.0;
        }

        let avg_power = self.power_accum[bin] / self.averaging as f32;
        if avg_power > 1e-20 {
            10.0 * avg_power.log10()
        } else {
            -120.0
        }
    }

    /// Get all bins as SpectrumBin array.
    pub fn get_spectrum(&self, output: &mut [SpectrumBin]) {
        let len = output.len().min(self.num_bins);
        for i in 0..len {
            output[i] = SpectrumBin {
                power_db: self.power_db(i),
            };
        }
    }

    /// Reset averaging accumulator.
    pub fn reset_averaging(&mut self) {
        self.power_accum = [0.0; MAX_BINS];
    }

    /// Full reset.
    pub fn reset(&mut self) {
        self.buffer = [0.0; MAX_BINS];
        self.write_pos = 0;
        self.real = [0.0; MAX_BINS];
        self.imag = [0.0; MAX_BINS];
        self.power_accum = [0.0; MAX_BINS];
        self.sample_count = 0;
    }
}

impl Default for SlidingDft {
    fn default() -> Self {
        Self::new(256, 4)
    }
}

/// Simple FFT-based spectrum analyzer.
///
/// Uses Cooley-Tukey radix-2 DIT FFT for power-of-2 sizes.
#[derive(Clone)]
pub struct FftSpectrum {
    /// FFT size (must be power of 2)
    size: usize,
    /// Sample buffer
    buffer: [f32; MAX_BINS],
    /// Real output
    real: [f32; MAX_BINS],
    /// Imaginary output
    imag: [f32; MAX_BINS],
    /// Write position
    write_pos: usize,
    /// Window function (Hann)
    window: [f32; MAX_BINS],
}

impl FftSpectrum {
    /// Create a new FFT spectrum analyzer.
    ///
    /// # Arguments
    /// * `size` - FFT size (must be power of 2, max 512)
    #[must_use]
    pub fn new(size: usize) -> Self {
        let size = size.min(MAX_BINS).next_power_of_two();

        // Precompute Hann window
        let mut window = [0.0; MAX_BINS];
        for i in 0..size {
            window[i] =
                0.5 * (1.0 - (2.0 * core::f32::consts::PI * i as f32 / size as f32).cos());
        }

        Self {
            size,
            buffer: [0.0; MAX_BINS],
            real: [0.0; MAX_BINS],
            imag: [0.0; MAX_BINS],
            write_pos: 0,
            window,
        }
    }

    /// Push a sample to the buffer.
    pub fn push(&mut self, sample: f32) {
        self.buffer[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.size;
    }

    /// Check if buffer is full and ready for FFT.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.write_pos == 0
    }

    /// Compute FFT and return power spectrum in dB.
    pub fn compute(&mut self, output: &mut [f32]) {
        // Apply window and copy to real buffer
        for i in 0..self.size {
            let idx = (self.write_pos + i) % self.size;
            self.real[i] = self.buffer[idx] * self.window[i];
            self.imag[i] = 0.0;
        }

        // In-place FFT
        self.fft_in_place();

        // Compute power spectrum
        let len = output.len().min(self.size / 2);
        for i in 0..len {
            let power = self.real[i] * self.real[i] + self.imag[i] * self.imag[i];
            output[i] = if power > 1e-20 {
                10.0 * power.log10()
            } else {
                -120.0
            };
        }
    }

    /// In-place radix-2 DIT FFT.
    fn fft_in_place(&mut self) {
        let n = self.size;

        // Bit-reverse permutation
        let mut j = 0;
        for i in 0..n - 1 {
            if i < j {
                self.real.swap(i, j);
                self.imag.swap(i, j);
            }
            let mut k = n / 2;
            while k <= j {
                j -= k;
                k /= 2;
            }
            j += k;
        }

        // Cooley-Tukey butterflies
        let mut len = 2;
        while len <= n {
            let half = len / 2;
            let angle_step = -2.0 * core::f32::consts::PI / len as f32;

            for i in (0..n).step_by(len) {
                let mut angle = 0.0;
                for j in 0..half {
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    let u_r = self.real[i + j];
                    let u_i = self.imag[i + j];
                    let t_r = cos_a * self.real[i + j + half] - sin_a * self.imag[i + j + half];
                    let t_i = sin_a * self.real[i + j + half] + cos_a * self.imag[i + j + half];

                    self.real[i + j] = u_r + t_r;
                    self.imag[i + j] = u_i + t_i;
                    self.real[i + j + half] = u_r - t_r;
                    self.imag[i + j + half] = u_i - t_i;

                    angle += angle_step;
                }
            }
            len *= 2;
        }
    }

    /// Reset analyzer.
    pub fn reset(&mut self) {
        self.buffer = [0.0; MAX_BINS];
        self.write_pos = 0;
    }
}

impl Default for FftSpectrum {
    fn default() -> Self {
        Self::new(256)
    }
}

/// Waterfall row data (for display).
#[derive(Clone, Debug)]
pub struct WaterfallRow {
    /// Timestamp (frame number)
    pub timestamp: u32,
    /// Power values (0-255, mapped from dB)
    pub data: [u8; MAX_BINS],
    /// Number of valid bins
    pub num_bins: usize,
}

impl WaterfallRow {
    /// Create a new waterfall row from spectrum data.
    #[must_use]
    pub fn from_spectrum(
        spectrum: &[SpectrumBin],
        timestamp: u32,
        ref_db: f32,
        range_db: f32,
    ) -> Self {
        let mut data = [0u8; MAX_BINS];
        let num_bins = spectrum.len().min(MAX_BINS);

        for (i, bin) in spectrum.iter().take(num_bins).enumerate() {
            // Map dB to 0-255
            let normalized = ((ref_db - bin.power_db) / range_db).clamp(0.0, 1.0);
            data[i] = (255.0 * (1.0 - normalized)) as u8;
        }

        Self {
            timestamp,
            data,
            num_bins,
        }
    }
}

impl Default for WaterfallRow {
    fn default() -> Self {
        Self {
            timestamp: 0,
            data: [0; MAX_BINS],
            num_bins: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectrum_config_bin_width() {
        let config = SpectrumConfig {
            fft_size: 256,
            sample_rate: 48000,
            ..Default::default()
        };

        assert!((config.bin_width() - 187.5).abs() < 0.1);
    }

    #[test]
    fn test_sliding_dft_detects_tone() {
        let mut dft = SlidingDft::new(64, 1);

        // Feed a 1000 Hz tone at 48000 Hz sample rate
        let freq = 1000.0;
        let sample_rate = 48000.0;

        for i in 0..256 {
            let sample = (2.0 * core::f32::consts::PI * freq * i as f32 / sample_rate).sin();
            dft.push(sample);
        }

        dft.compute();

        // Find peak bin (should be around bin 1-2 for 64 bins at 48kHz)
        // Bin width = 48000/64 = 750 Hz
        // 1000 Hz / 750 Hz = bin ~1.33
        let bin1_power = dft.power_db(1);
        let bin2_power = dft.power_db(2);

        // Peak should be significantly higher than other bins
        let noise_power = dft.power_db(32);
        assert!(bin1_power > noise_power + 10.0 || bin2_power > noise_power + 10.0);
    }

    #[test]
    fn test_waterfall_row_mapping() {
        let spectrum = [
            SpectrumBin { power_db: 0.0 },   // Max signal
            SpectrumBin { power_db: -40.0 }, // Mid
            SpectrumBin { power_db: -80.0 }, // Min
        ];

        let row = WaterfallRow::from_spectrum(&spectrum, 0, 0.0, 80.0);

        assert_eq!(row.data[0], 255); // 0 dB = max brightness
        assert!(row.data[1] > 100 && row.data[1] < 200); // -40 dB = mid
        assert_eq!(row.data[2], 0); // -80 dB = min brightness
    }

    #[test]
    fn test_fft_power_of_two() {
        let fft = FftSpectrum::new(100); // Should round up to 128
        assert!(fft.size.is_power_of_two());
    }
}
