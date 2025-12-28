//! Spectrum Analysis Module
//!
//! Provides spectrum analysis and waterfall display data generation.
//! Uses efficient algorithms suitable for embedded real-time processing.

#[cfg(feature = "embedded")]
use micromath::F32Ext;

/// Power spectrum bin for display
#[derive(Clone, Copy, Debug, Default)]
pub struct SpectrumBin {
    /// Frequency in Hz
    pub frequency: u32,
    /// Power level in dB (relative to full scale)
    pub power_db: f32,
}

/// Spectrum analyzer configuration
#[derive(Clone, Copy, Debug)]
pub struct SpectrumConfig {
    /// Center frequency in Hz
    pub center_freq: u32,
    /// Span in Hz
    pub span_hz: u32,
    /// Number of FFT bins
    pub fft_size: usize,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Averaging count (1 = no averaging)
    pub averaging: u8,
}

impl Default for SpectrumConfig {
    fn default() -> Self {
        Self {
            center_freq: 7_100_000,
            span_hz: 48000,
            fft_size: 256,
            sample_rate: 48000,
            averaging: 4,
        }
    }
}

impl SpectrumConfig {
    /// Calculate frequency resolution (Hz per bin)
    #[must_use]
    pub fn bin_width(&self) -> f32 {
        self.sample_rate as f32 / self.fft_size as f32
    }

    /// Calculate frequency for a given bin index
    #[must_use]
    pub fn bin_frequency(&self, bin: usize) -> u32 {
        let offset = (bin as i32 - self.fft_size as i32 / 2) as f32 * self.bin_width();
        (self.center_freq as f32 + offset) as u32
    }
}

/// Simple sliding DFT for efficient bin-by-bin computation
///
/// More efficient than full FFT when only a few bins are needed.
#[derive(Clone)]
pub struct SlidingDft {
    /// Number of bins to compute
    num_bins: usize,
    /// Sliding window buffer
    buffer: [f32; 256],
    /// Current position in buffer
    pos: usize,
    /// Twiddle factors (cos) per bin - 32 bins x 256 samples
    twiddles: [[f32; 256]; 32],
    /// Accumulated power per bin
    power: [f32; 32],
    /// Sample count for averaging
    sample_count: u32,
}

impl SlidingDft {
    /// Create a new sliding DFT analyzer
    ///
    /// # Arguments
    /// * `num_bins` - Number of frequency bins to compute (max 32)
    /// * `window_size` - Window size (256)
    #[must_use]
    pub fn new(num_bins: usize) -> Self {
        let num_bins = num_bins.min(32);
        let mut twiddles = [([0.0f32; 256]); 32];

        // Precompute twiddle factors
        for k in 0..num_bins {
            for n in 0..256 {
                let angle = 2.0 * core::f32::consts::PI * (k as f32) * (n as f32) / 256.0;
                twiddles[k][n] = angle.cos();
            }
        }

        Self {
            num_bins,
            buffer: [0.0; 256],
            pos: 0,
            twiddles,
            power: [0.0; 32],
            sample_count: 0,
        }
    }

    /// Add a sample to the sliding window
    pub fn push(&mut self, sample: f32) {
        self.buffer[self.pos] = sample;
        self.pos = (self.pos + 1) & 0xFF;
    }

    /// Compute power for all configured bins
    pub fn compute(&mut self) {
        for k in 0..self.num_bins {
            let mut real = 0.0f32;
            let mut imag = 0.0f32;

            // Manual DFT computation for this bin
            for n in 0..256 {
                let idx = (self.pos + n) & 0xFF;
                let cos_val = self.twiddles[k][n];
                // sin = cos(x - π/2), approximate with phase shift
                let sin_val = self.twiddles[k][(n + 64) & 0xFF];

                real += self.buffer[idx] * cos_val;
                imag += self.buffer[idx] * sin_val;
            }

            // Power = |X|^2
            let pwr = real * real + imag * imag;
            self.power[k] += pwr;
        }
        self.sample_count += 1;
    }

    /// Get power in dB for a bin (with averaging)
    #[must_use]
    pub fn power_db(&self, bin: usize) -> f32 {
        if bin >= self.num_bins || self.sample_count == 0 {
            return -100.0;
        }

        let avg_power = self.power[bin] / self.sample_count as f32;
        // Convert to dB, with floor
        if avg_power < 1e-10 {
            -100.0
        } else {
            10.0 * avg_power.log10()
        }
    }

    /// Reset accumulator for new measurement
    pub fn reset(&mut self) {
        self.power.fill(0.0);
        self.sample_count = 0;
    }

    /// Get number of bins
    #[must_use]
    pub fn num_bins(&self) -> usize {
        self.num_bins
    }
}

impl Default for SlidingDft {
    fn default() -> Self {
        Self::new(32)
    }
}

/// Peak detector for spectrum display
#[derive(Clone, Copy, Debug, Default)]
pub struct PeakDetector {
    /// Peak frequency in Hz
    pub peak_freq: u32,
    /// Peak power in dB
    pub peak_power: f32,
    /// Noise floor estimate in dB
    pub noise_floor: f32,
}

impl PeakDetector {
    /// Find peak in spectrum data
    #[must_use]
    pub fn find_peak(bins: &[SpectrumBin]) -> Self {
        if bins.is_empty() {
            return Self::default();
        }

        let mut peak_idx = 0;
        let mut peak_power = f32::NEG_INFINITY;
        let mut sum_power = 0.0f32;

        for (i, bin) in bins.iter().enumerate() {
            if bin.power_db > peak_power {
                peak_power = bin.power_db;
                peak_idx = i;
            }
            sum_power += bin.power_db;
        }

        let noise_floor = sum_power / bins.len() as f32;

        Self {
            peak_freq: bins[peak_idx].frequency,
            peak_power,
            noise_floor,
        }
    }

    /// Check if peak is significant (above noise floor)
    #[must_use]
    pub fn is_significant(&self, threshold_db: f32) -> bool {
        self.peak_power > self.noise_floor + threshold_db
    }
}

/// Waterfall display row
#[derive(Clone, Debug)]
pub struct WaterfallRow {
    /// Timestamp (sample count or milliseconds)
    pub timestamp: u32,
    /// Power values in dB for each column
    pub data: [i8; 128],
}

impl WaterfallRow {
    /// Create from spectrum bins
    #[must_use]
    pub fn from_spectrum(timestamp: u32, bins: &[SpectrumBin], num_columns: usize) -> Self {
        let mut data = [0i8; 128];
        let columns = num_columns.min(128);

        if !bins.is_empty() {
            let bins_per_col = bins.len() / columns;

            for col in 0..columns {
                let start = col * bins_per_col;
                let end = ((col + 1) * bins_per_col).min(bins.len());

                // Find max power in this column's range
                let max_power = bins[start..end]
                    .iter()
                    .map(|b| b.power_db)
                    .fold(f32::NEG_INFINITY, f32::max);

                // Convert to i8 (-100 dB to 0 dB range)
                data[col] = (max_power.clamp(-100.0, 0.0) + 100.0) as i8;
            }
        }

        Self { timestamp, data }
    }

    /// Get power at column (0-100 range for display)
    #[must_use]
    pub fn power_at(&self, col: usize) -> u8 {
        if col < 128 {
            self.data[col] as u8
        } else {
            0
        }
    }
}

impl Default for WaterfallRow {
    fn default() -> Self {
        Self {
            timestamp: 0,
            data: [0; 128],
        }
    }
}

/// Waterfall display buffer (circular buffer of rows)
pub struct WaterfallBuffer<const ROWS: usize> {
    rows: [WaterfallRow; ROWS],
    head: usize,
    count: usize,
}

impl<const ROWS: usize> WaterfallBuffer<ROWS> {
    /// Create a new waterfall buffer
    #[must_use]
    pub fn new() -> Self {
        Self {
            rows: core::array::from_fn(|_| WaterfallRow::default()),
            head: 0,
            count: 0,
        }
    }

    /// Push a new row (oldest row is dropped)
    pub fn push(&mut self, row: WaterfallRow) {
        self.rows[self.head] = row;
        self.head = (self.head + 1) % ROWS;
        if self.count < ROWS {
            self.count += 1;
        }
    }

    /// Get row by index (0 = most recent)
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&WaterfallRow> {
        if index >= self.count {
            return None;
        }
        let actual_idx = (self.head + ROWS - 1 - index) % ROWS;
        Some(&self.rows[actual_idx])
    }

    /// Number of rows in buffer
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if buffer is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.head = 0;
        self.count = 0;
    }
}

impl<const ROWS: usize> Default for WaterfallBuffer<ROWS> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    // =========================================================================
    // Spectrum Config Tests
    // =========================================================================

    #[test]
    fn config_default() {
        let config = SpectrumConfig::default();
        assert_eq!(config.fft_size, 256);
        assert_eq!(config.sample_rate, 48000);
    }

    #[test]
    fn config_bin_width() {
        let config = SpectrumConfig::default();
        let width = config.bin_width();
        // 48000 / 256 = 187.5 Hz
        assert!((width - 187.5).abs() < 0.1);
    }

    #[test]
    fn config_bin_frequency_center() {
        let config = SpectrumConfig::default();
        // Center bin should be center frequency
        let center_bin = config.fft_size / 2;
        let freq = config.bin_frequency(center_bin);
        assert_eq!(freq, config.center_freq);
    }

    // =========================================================================
    // Sliding DFT Tests
    // =========================================================================

    #[test]
    fn sliding_dft_creation() {
        let dft = SlidingDft::new(16);
        assert_eq!(dft.num_bins(), 16);
    }

    #[test]
    fn sliding_dft_default() {
        let dft = SlidingDft::default();
        assert_eq!(dft.num_bins(), 32);
    }

    #[test]
    fn sliding_dft_push() {
        let mut dft = SlidingDft::new(8);
        for i in 0..256 {
            dft.push(i as f32 / 256.0);
        }
        // Should not panic
    }

    #[test]
    fn sliding_dft_compute() {
        let mut dft = SlidingDft::new(8);

        // Fill with sine wave
        for i in 0..256 {
            let sample = (2.0 * core::f32::consts::PI * 4.0 * i as f32 / 256.0).sin();
            dft.push(sample);
        }

        dft.compute();

        // Check that we get finite power values
        for bin in 0..8 {
            let power = dft.power_db(bin);
            assert!(power.is_finite(), "Bin {} power should be finite", bin);
        }
    }

    #[test]
    fn sliding_dft_reset() {
        let mut dft = SlidingDft::new(8);

        for i in 0..256 {
            dft.push(i as f32 / 256.0);
        }
        dft.compute();
        dft.reset();

        // After reset, power should be very low
        let power = dft.power_db(0);
        assert_eq!(power, -100.0);
    }

    #[test]
    fn sliding_dft_max_bins() {
        let dft = SlidingDft::new(100);
        // Should clamp to 32
        assert_eq!(dft.num_bins(), 32);
    }

    // =========================================================================
    // Peak Detector Tests
    // =========================================================================

    #[test]
    fn peak_detector_empty() {
        let peak = PeakDetector::find_peak(&[]);
        assert_eq!(peak.peak_freq, 0);
    }

    #[test]
    fn peak_detector_single() {
        let bins = [SpectrumBin {
            frequency: 1000,
            power_db: -30.0,
        }];
        let peak = PeakDetector::find_peak(&bins);
        assert_eq!(peak.peak_freq, 1000);
        assert_eq!(peak.peak_power, -30.0);
    }

    #[test]
    fn peak_detector_finds_max() {
        let bins = [
            SpectrumBin {
                frequency: 1000,
                power_db: -40.0,
            },
            SpectrumBin {
                frequency: 2000,
                power_db: -20.0,
            },
            SpectrumBin {
                frequency: 3000,
                power_db: -30.0,
            },
        ];
        let peak = PeakDetector::find_peak(&bins);
        assert_eq!(peak.peak_freq, 2000);
        assert_eq!(peak.peak_power, -20.0);
    }

    #[test]
    fn peak_detector_significance() {
        let peak = PeakDetector {
            peak_freq: 1000,
            peak_power: -20.0,
            noise_floor: -40.0,
        };
        assert!(peak.is_significant(10.0)); // 20 dB above noise
        assert!(!peak.is_significant(30.0)); // Not 30 dB above
    }

    // =========================================================================
    // Waterfall Row Tests
    // =========================================================================

    #[test]
    fn waterfall_row_default() {
        let row = WaterfallRow::default();
        assert_eq!(row.timestamp, 0);
    }

    #[test]
    fn waterfall_row_from_spectrum() {
        let bins = vec![
            SpectrumBin {
                frequency: 1000,
                power_db: -30.0,
            },
            SpectrumBin {
                frequency: 2000,
                power_db: -40.0,
            },
        ];
        let row = WaterfallRow::from_spectrum(100, &bins, 2);
        assert_eq!(row.timestamp, 100);
    }

    #[test]
    fn waterfall_row_power_at() {
        let row = WaterfallRow::default();
        assert_eq!(row.power_at(0), 0);
        assert_eq!(row.power_at(200), 0); // Out of bounds
    }

    // =========================================================================
    // Waterfall Buffer Tests
    // =========================================================================

    #[test]
    fn waterfall_buffer_creation() {
        let buffer: WaterfallBuffer<64> = WaterfallBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn waterfall_buffer_push_get() {
        let mut buffer: WaterfallBuffer<4> = WaterfallBuffer::new();

        for i in 0..3 {
            buffer.push(WaterfallRow {
                timestamp: i,
                data: [i as i8; 128],
            });
        }

        assert_eq!(buffer.len(), 3);

        // Most recent (index 0) should be timestamp 2
        let row = buffer.get(0).unwrap();
        assert_eq!(row.timestamp, 2);

        // Oldest should be timestamp 0
        let row = buffer.get(2).unwrap();
        assert_eq!(row.timestamp, 0);
    }

    #[test]
    fn waterfall_buffer_overflow() {
        let mut buffer: WaterfallBuffer<4> = WaterfallBuffer::new();

        for i in 0..10 {
            buffer.push(WaterfallRow {
                timestamp: i,
                data: [0; 128],
            });
        }

        // Should have max 4 rows
        assert_eq!(buffer.len(), 4);

        // Most recent should be 9
        let row = buffer.get(0).unwrap();
        assert_eq!(row.timestamp, 9);

        // Oldest should be 6
        let row = buffer.get(3).unwrap();
        assert_eq!(row.timestamp, 6);
    }

    #[test]
    fn waterfall_buffer_clear() {
        let mut buffer: WaterfallBuffer<4> = WaterfallBuffer::new();

        buffer.push(WaterfallRow::default());
        buffer.push(WaterfallRow::default());

        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn waterfall_buffer_get_invalid() {
        let buffer: WaterfallBuffer<4> = WaterfallBuffer::new();
        assert!(buffer.get(0).is_none());
    }
}
