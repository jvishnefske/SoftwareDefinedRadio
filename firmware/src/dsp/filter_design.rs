//! Filter Design Module
//!
//! Provides coefficient calculation for various filter types used in
//! the SDR transceiver. All calculations are done at compile time or
//! initialization, not during real-time audio processing.
//!
//! # Supported Filter Types
//!
//! - Biquad (IIR): Low-pass, high-pass, band-pass, notch, peaking EQ
//! - CW filter: Narrow band-pass for Morse reception
//! - SSB filter: 2.4 kHz bandwidth for voice
//! - AM filter: 6 kHz bandwidth

use core::f32::consts::PI;

#[cfg(feature = "embedded")]
use micromath::F32Ext;

/// Biquad filter coefficients (Direct Form I)
///
/// Transfer function: H(z) = (b0 + b1*z^-1 + b2*z^-2) / (1 + a1*z^-1 + a2*z^-2)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiquadCoeffs {
    /// Numerator coefficient b0
    pub b0: f32,
    /// Numerator coefficient b1
    pub b1: f32,
    /// Numerator coefficient b2
    pub b2: f32,
    /// Denominator coefficient a1 (note: a0 is normalized to 1)
    pub a1: f32,
    /// Denominator coefficient a2
    pub a2: f32,
}

impl BiquadCoeffs {
    /// Unity (pass-through) coefficients
    pub const UNITY: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    /// Design a low-pass filter
    ///
    /// # Arguments
    /// * `fc` - Cutoff frequency in Hz
    /// * `fs` - Sample rate in Hz
    /// * `q` - Quality factor (0.707 for Butterworth)
    #[must_use]
    pub fn lowpass(fc: f32, fs: f32, q: f32) -> Self {
        let omega = 2.0 * PI * fc / fs;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = (1.0 - cos_omega) / 2.0;
        let b1 = 1.0 - cos_omega;
        let b2 = (1.0 - cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Design a high-pass filter
    ///
    /// # Arguments
    /// * `fc` - Cutoff frequency in Hz
    /// * `fs` - Sample rate in Hz
    /// * `q` - Quality factor (0.707 for Butterworth)
    #[must_use]
    pub fn highpass(fc: f32, fs: f32, q: f32) -> Self {
        let omega = 2.0 * PI * fc / fs;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = f32::midpoint(1.0, cos_omega);
        let b1 = -(1.0 + cos_omega);
        let b2 = f32::midpoint(1.0, cos_omega);
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Design a band-pass filter (constant skirt gain)
    ///
    /// # Arguments
    /// * `fc` - Center frequency in Hz
    /// * `fs` - Sample rate in Hz
    /// * `q` - Quality factor (bandwidth = fc/Q)
    #[must_use]
    pub fn bandpass(fc: f32, fs: f32, q: f32) -> Self {
        let omega = 2.0 * PI * fc / fs;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Design a band-pass filter (constant peak gain)
    ///
    /// # Arguments
    /// * `fc` - Center frequency in Hz
    /// * `fs` - Sample rate in Hz
    /// * `q` - Quality factor
    #[must_use]
    pub fn bandpass_peak(fc: f32, fs: f32, q: f32) -> Self {
        let omega = 2.0 * PI * fc / fs;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = sin_omega / 2.0;
        let b1 = 0.0;
        let b2 = -sin_omega / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Design a notch (band-reject) filter
    ///
    /// # Arguments
    /// * `fc` - Center frequency in Hz
    /// * `fs` - Sample rate in Hz
    /// * `q` - Quality factor (higher = narrower notch)
    #[must_use]
    pub fn notch(fc: f32, fs: f32, q: f32) -> Self {
        let omega = 2.0 * PI * fc / fs;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = 1.0;
        let b1 = -2.0 * cos_omega;
        let b2 = 1.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Design a peaking EQ filter
    ///
    /// # Arguments
    /// * `fc` - Center frequency in Hz
    /// * `fs` - Sample rate in Hz
    /// * `q` - Quality factor
    /// * `gain_db` - Gain at center frequency in dB
    #[must_use]
    pub fn peaking_eq(fc: f32, fs: f32, q: f32, gain_db: f32) -> Self {
        let omega = 2.0 * PI * fc / fs;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = sin_omega / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_omega;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha / a;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Design a low-shelf filter
    ///
    /// # Arguments
    /// * `fc` - Corner frequency in Hz
    /// * `fs` - Sample rate in Hz
    /// * `gain_db` - Shelf gain in dB
    /// * `s` - Slope parameter (1.0 for 6dB/oct)
    #[must_use]
    pub fn low_shelf(fc: f32, fs: f32, gain_db: f32, s: f32) -> Self {
        let omega = 2.0 * PI * fc / fs;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let beta = (a.powi(2) + 1.0) / s - (a - 1.0).powi(2);
        let beta = if beta > 0.0 { beta.sqrt() } else { 0.0 };

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_omega + beta * sin_omega);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_omega);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_omega - beta * sin_omega);
        let a0 = (a + 1.0) + (a - 1.0) * cos_omega + beta * sin_omega;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_omega);
        let a2 = (a + 1.0) + (a - 1.0) * cos_omega - beta * sin_omega;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Design a high-shelf filter
    ///
    /// # Arguments
    /// * `fc` - Corner frequency in Hz
    /// * `fs` - Sample rate in Hz
    /// * `gain_db` - Shelf gain in dB
    /// * `s` - Slope parameter (1.0 for 6dB/oct)
    #[must_use]
    pub fn high_shelf(fc: f32, fs: f32, gain_db: f32, s: f32) -> Self {
        let omega = 2.0 * PI * fc / fs;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let beta = (a.powi(2) + 1.0) / s - (a - 1.0).powi(2);
        let beta = if beta > 0.0 { beta.sqrt() } else { 0.0 };

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_omega + beta * sin_omega);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_omega);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_omega - beta * sin_omega);
        let a0 = (a + 1.0) - (a - 1.0) * cos_omega + beta * sin_omega;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_omega);
        let a2 = (a + 1.0) - (a - 1.0) * cos_omega - beta * sin_omega;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Normalize coefficients by a0
    fn normalize(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Calculate magnitude response at a given frequency
    #[must_use]
    pub fn magnitude_at(&self, freq: f32, fs: f32) -> f32 {
        let omega = 2.0 * PI * freq / fs;
        let cos_omega = omega.cos();
        let cos_2omega = (2.0 * omega).cos();

        // |H(e^jw)|^2 = |B(e^jw)|^2 / |A(e^jw)|^2
        let num = self.b0 * self.b0 + self.b1 * self.b1 + self.b2 * self.b2
            + 2.0 * (self.b0 * self.b1 + self.b1 * self.b2) * cos_omega
            + 2.0 * self.b0 * self.b2 * cos_2omega;

        let den = 1.0 + self.a1 * self.a1 + self.a2 * self.a2
            + 2.0 * (self.a1 + self.a1 * self.a2) * cos_omega
            + 2.0 * self.a2 * cos_2omega;

        if den > 0.0 {
            (num / den).sqrt()
        } else {
            0.0
        }
    }

    /// Calculate magnitude response in dB at a given frequency
    #[must_use]
    pub fn magnitude_db_at(&self, freq: f32, fs: f32) -> f32 {
        let mag = self.magnitude_at(freq, fs);
        if mag > 0.0 {
            20.0 * mag.log10()
        } else {
            -120.0
        }
    }
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        Self::UNITY
    }
}

/// CW filter bandwidth options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CwBandwidth {
    /// 50 Hz (contest)
    Hz50,
    /// 100 Hz (narrow)
    Hz100,
    /// 200 Hz (medium)
    Hz200,
    /// 400 Hz (wide)
    #[default]
    Hz400,
    /// 800 Hz (very wide)
    Hz800,
}

impl CwBandwidth {
    /// Get bandwidth in Hz
    #[must_use]
    pub const fn hz(&self) -> u16 {
        match self {
            Self::Hz50 => 50,
            Self::Hz100 => 100,
            Self::Hz200 => 200,
            Self::Hz400 => 400,
            Self::Hz800 => 800,
        }
    }

    /// Calculate Q factor for given bandwidth
    #[must_use]
    pub fn q_at(&self, center_freq: f32) -> f32 {
        center_freq / f32::from(self.hz())
    }
}

/// SSB filter bandwidth options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SsbBandwidth {
    /// 1.8 kHz (narrow)
    Narrow,
    /// 2.4 kHz (standard)
    #[default]
    Standard,
    /// 2.7 kHz (wide)
    Wide,
    /// 3.0 kHz (extra wide)
    ExtraWide,
}

impl SsbBandwidth {
    /// Get low cutoff frequency in Hz
    #[must_use]
    pub const fn low_cutoff(&self) -> u16 {
        match self {
            Self::Narrow => 400,
            Self::Standard => 300,
            Self::Wide => 200,
            Self::ExtraWide => 100,
        }
    }

    /// Get high cutoff frequency in Hz
    #[must_use]
    pub const fn high_cutoff(&self) -> u16 {
        match self {
            Self::Narrow => 2200,
            Self::Standard => 2700,
            Self::Wide => 2900,
            Self::ExtraWide => 3100,
        }
    }

    /// Get bandwidth in Hz
    #[must_use]
    pub const fn bandwidth(&self) -> u16 {
        self.high_cutoff() - self.low_cutoff()
    }
}

/// AM filter bandwidth options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AmBandwidth {
    /// 3 kHz (narrow)
    Narrow,
    /// 6 kHz (standard)
    #[default]
    Standard,
    /// 9 kHz (wide - broadcast)
    Wide,
}

impl AmBandwidth {
    /// Get bandwidth in Hz
    #[must_use]
    pub const fn hz(&self) -> u16 {
        match self {
            Self::Narrow => 3000,
            Self::Standard => 6000,
            Self::Wide => 9000,
        }
    }
}

/// Design a CW audio filter (single biquad stage)
///
/// For sharper response, cascade multiple stages
#[must_use]
pub fn design_cw_filter(center_freq: f32, bandwidth: CwBandwidth, sample_rate: f32) -> BiquadCoeffs {
    let q = bandwidth.q_at(center_freq);
    BiquadCoeffs::bandpass_peak(center_freq, sample_rate, q)
}

/// Design an SSB audio filter (cascaded high-pass and low-pass)
///
/// Returns (high-pass coeffs, low-pass coeffs)
#[must_use]
pub fn design_ssb_filter(bandwidth: SsbBandwidth, sample_rate: f32) -> (BiquadCoeffs, BiquadCoeffs) {
    let q = 0.707; // Butterworth response

    let hpf = BiquadCoeffs::highpass(f32::from(bandwidth.low_cutoff()), sample_rate, q);
    let lpf = BiquadCoeffs::lowpass(f32::from(bandwidth.high_cutoff()), sample_rate, q);

    (hpf, lpf)
}

/// Design an AM audio filter (low-pass only)
#[must_use]
pub fn design_am_filter(bandwidth: AmBandwidth, sample_rate: f32) -> BiquadCoeffs {
    let q = 0.707; // Butterworth response
    let cutoff = f32::from(bandwidth.hz()) / 2.0; // Single sideband cutoff

    BiquadCoeffs::lowpass(cutoff, sample_rate, q)
}

/// Design a de-emphasis filter for FM audio
///
/// Standard 75µs or 50µs time constant
#[must_use]
pub fn design_deemphasis_filter(time_constant_us: f32, sample_rate: f32) -> BiquadCoeffs {
    // Corner frequency = 1 / (2π × τ)
    let fc = 1_000_000.0 / (2.0 * PI * time_constant_us);
    BiquadCoeffs::lowpass(fc, sample_rate, 0.707)
}

/// Design a pre-emphasis filter for FM transmission
///
/// Standard 75µs or 50µs time constant
#[must_use]
pub fn design_preemphasis_filter(time_constant_us: f32, sample_rate: f32) -> BiquadCoeffs {
    let fc = 1_000_000.0 / (2.0 * PI * time_constant_us);
    BiquadCoeffs::highpass(fc, sample_rate, 0.707)
}

/// Design a DC blocking filter
#[must_use]
pub fn design_dc_blocker(sample_rate: f32) -> BiquadCoeffs {
    // Very low cutoff high-pass filter
    BiquadCoeffs::highpass(10.0, sample_rate, 0.707)
}

/// Design a noise blanker threshold filter
#[must_use]
pub fn design_noise_blanker_lpf(sample_rate: f32) -> BiquadCoeffs {
    // Low-pass to smooth the envelope detector
    BiquadCoeffs::lowpass(2000.0, sample_rate, 0.707)
}

/// Biquad filter state using `filter_design` coefficients
///
/// Implements Direct Form II Transposed for numerical stability.
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    coeffs: BiquadCoeffs,
    /// State variables
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// Create a new biquad filter with given coefficients
    #[must_use]
    pub fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            coeffs,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// Process a single sample through the filter
    pub fn process(&mut self, input: f32) -> f32 {
        // Direct Form II Transposed
        let output = self.coeffs.b0 * input + self.z1;
        self.z1 = self.coeffs.b1 * input - self.coeffs.a1 * output + self.z2;
        self.z2 = self.coeffs.b2 * input - self.coeffs.a2 * output;
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
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    /// Update coefficients (preserves state)
    pub fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }

    /// Get current coefficients
    #[must_use]
    pub fn coeffs(&self) -> BiquadCoeffs {
        self.coeffs
    }
}

impl Default for Biquad {
    fn default() -> Self {
        Self::new(BiquadCoeffs::UNITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48000.0;
    const TOLERANCE: f32 = 0.01; // 1% tolerance for magnitude tests

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn biquad_unity() {
        let coeffs = BiquadCoeffs::UNITY;
        assert_eq!(coeffs.b0, 1.0);
        assert_eq!(coeffs.b1, 0.0);
        assert_eq!(coeffs.b2, 0.0);
        assert_eq!(coeffs.a1, 0.0);
        assert_eq!(coeffs.a2, 0.0);
    }

    #[test]
    fn biquad_lowpass_response() {
        let fc = 1000.0;
        let coeffs = BiquadCoeffs::lowpass(fc, SAMPLE_RATE, 0.707);

        // At DC, magnitude should be ~1.0
        let mag_dc = coeffs.magnitude_at(10.0, SAMPLE_RATE);
        assert!(approx_eq(mag_dc, 1.0, TOLERANCE), "DC magnitude: {}", mag_dc);

        // At cutoff, magnitude should be ~0.707 (-3dB)
        let mag_fc = coeffs.magnitude_at(fc, SAMPLE_RATE);
        assert!(
            approx_eq(mag_fc, 0.707, 0.05),
            "Cutoff magnitude: {}",
            mag_fc
        );

        // Well above cutoff, magnitude should be low
        let mag_high = coeffs.magnitude_at(fc * 10.0, SAMPLE_RATE);
        assert!(mag_high < 0.1, "High freq magnitude: {}", mag_high);
    }

    #[test]
    fn biquad_highpass_response() {
        let fc = 1000.0;
        let coeffs = BiquadCoeffs::highpass(fc, SAMPLE_RATE, 0.707);

        // At DC, magnitude should be ~0
        let mag_dc = coeffs.magnitude_at(10.0, SAMPLE_RATE);
        assert!(mag_dc < 0.1, "DC magnitude: {}", mag_dc);

        // At cutoff, magnitude should be ~0.707 (-3dB)
        let mag_fc = coeffs.magnitude_at(fc, SAMPLE_RATE);
        assert!(
            approx_eq(mag_fc, 0.707, 0.05),
            "Cutoff magnitude: {}",
            mag_fc
        );

        // Well above cutoff, magnitude should be ~1.0
        let mag_high = coeffs.magnitude_at(fc * 10.0, SAMPLE_RATE);
        assert!(
            approx_eq(mag_high, 1.0, TOLERANCE),
            "High freq magnitude: {}",
            mag_high
        );
    }

    #[test]
    fn biquad_bandpass_response() {
        let fc = 1000.0;
        let q = 10.0;
        let coeffs = BiquadCoeffs::bandpass(fc, SAMPLE_RATE, q);

        // At center, magnitude should be peak
        let mag_center = coeffs.magnitude_at(fc, SAMPLE_RATE);
        assert!(mag_center > 0.5, "Center magnitude: {}", mag_center);

        // At DC, magnitude should be low
        let mag_dc = coeffs.magnitude_at(10.0, SAMPLE_RATE);
        assert!(mag_dc < 0.1, "DC magnitude: {}", mag_dc);

        // Well above center, magnitude should be low
        let mag_high = coeffs.magnitude_at(fc * 10.0, SAMPLE_RATE);
        assert!(mag_high < 0.2, "High freq magnitude: {}", mag_high);
    }

    #[test]
    fn biquad_notch_response() {
        let fc = 1000.0;
        let q = 10.0;
        let coeffs = BiquadCoeffs::notch(fc, SAMPLE_RATE, q);

        // At center, magnitude should be very low
        let mag_center = coeffs.magnitude_at(fc, SAMPLE_RATE);
        assert!(mag_center < 0.1, "Center magnitude: {}", mag_center);

        // Away from center, magnitude should be ~1.0
        let mag_away = coeffs.magnitude_at(fc * 2.0, SAMPLE_RATE);
        assert!(
            approx_eq(mag_away, 1.0, 0.1),
            "Away magnitude: {}",
            mag_away
        );
    }

    #[test]
    fn biquad_peaking_eq() {
        let fc = 1000.0;
        let q = 2.0;
        let gain_db = 6.0;
        let coeffs = BiquadCoeffs::peaking_eq(fc, SAMPLE_RATE, q, gain_db);

        // At center, magnitude should be ~2.0 (+6dB)
        let mag_center = coeffs.magnitude_at(fc, SAMPLE_RATE);
        let expected = 10.0_f32.powf(gain_db / 20.0);
        assert!(
            approx_eq(mag_center, expected, 0.1),
            "Center magnitude: {} (expected {})",
            mag_center,
            expected
        );

        // Away from center, magnitude should be ~1.0
        let mag_away = coeffs.magnitude_at(fc * 4.0, SAMPLE_RATE);
        assert!(
            approx_eq(mag_away, 1.0, 0.1),
            "Away magnitude: {}",
            mag_away
        );
    }

    #[test]
    fn biquad_magnitude_db() {
        let coeffs = BiquadCoeffs::lowpass(1000.0, SAMPLE_RATE, 0.707);

        // At cutoff, should be about -3dB
        let db = coeffs.magnitude_db_at(1000.0, SAMPLE_RATE);
        assert!(approx_eq(db, -3.0, 0.5), "Cutoff dB: {}", db);
    }

    #[test]
    fn cw_bandwidth_values() {
        assert_eq!(CwBandwidth::Hz50.hz(), 50);
        assert_eq!(CwBandwidth::Hz100.hz(), 100);
        assert_eq!(CwBandwidth::Hz200.hz(), 200);
        assert_eq!(CwBandwidth::Hz400.hz(), 400);
        assert_eq!(CwBandwidth::Hz800.hz(), 800);
    }

    #[test]
    fn cw_bandwidth_q() {
        let bw = CwBandwidth::Hz400;
        let q = bw.q_at(700.0);
        assert!(approx_eq(q, 1.75, 0.01), "Q factor: {}", q);
    }

    #[test]
    fn ssb_bandwidth_values() {
        let bw = SsbBandwidth::Standard;
        assert_eq!(bw.low_cutoff(), 300);
        assert_eq!(bw.high_cutoff(), 2700);
        assert_eq!(bw.bandwidth(), 2400);
    }

    #[test]
    fn am_bandwidth_values() {
        assert_eq!(AmBandwidth::Narrow.hz(), 3000);
        assert_eq!(AmBandwidth::Standard.hz(), 6000);
        assert_eq!(AmBandwidth::Wide.hz(), 9000);
    }

    #[test]
    fn design_cw_filter_test() {
        let coeffs = design_cw_filter(700.0, CwBandwidth::Hz400, SAMPLE_RATE);

        // Should pass center frequency
        let mag_center = coeffs.magnitude_at(700.0, SAMPLE_RATE);
        assert!(mag_center > 0.5, "Center magnitude: {}", mag_center);

        // Should attenuate outside passband (single biquad: ~12 dB/oct rolloff)
        let mag_outside = coeffs.magnitude_at(2000.0, SAMPLE_RATE);
        assert!(mag_outside < 0.5, "Outside magnitude: {}", mag_outside);

        // Much further out should be more attenuated
        let mag_far = coeffs.magnitude_at(5000.0, SAMPLE_RATE);
        assert!(mag_far < 0.2, "Far outside magnitude: {}", mag_far);
    }

    #[test]
    fn design_ssb_filter_test() {
        let (hpf, lpf) = design_ssb_filter(SsbBandwidth::Standard, SAMPLE_RATE);

        // HPF should pass 1000 Hz
        let hpf_mag = hpf.magnitude_at(1000.0, SAMPLE_RATE);
        assert!(hpf_mag > 0.9, "HPF @ 1kHz: {}", hpf_mag);

        // HPF should block DC
        let hpf_dc = hpf.magnitude_at(50.0, SAMPLE_RATE);
        assert!(hpf_dc < 0.3, "HPF @ DC: {}", hpf_dc);

        // LPF should pass 1000 Hz
        let lpf_mag = lpf.magnitude_at(1000.0, SAMPLE_RATE);
        assert!(lpf_mag > 0.9, "LPF @ 1kHz: {}", lpf_mag);

        // LPF should attenuate 5000 Hz
        let lpf_high = lpf.magnitude_at(5000.0, SAMPLE_RATE);
        assert!(lpf_high < 0.3, "LPF @ 5kHz: {}", lpf_high);
    }

    #[test]
    fn design_am_filter_test() {
        let coeffs = design_am_filter(AmBandwidth::Standard, SAMPLE_RATE);

        // Should pass 1000 Hz
        let mag_pass = coeffs.magnitude_at(1000.0, SAMPLE_RATE);
        assert!(mag_pass > 0.9, "Pass magnitude: {}", mag_pass);

        // Should attenuate above bandwidth
        let mag_stop = coeffs.magnitude_at(6000.0, SAMPLE_RATE);
        assert!(mag_stop < 0.3, "Stop magnitude: {}", mag_stop);
    }

    #[test]
    fn design_deemphasis_test() {
        let coeffs = design_deemphasis_filter(75.0, SAMPLE_RATE);

        // Should be unity at low frequencies
        let mag_low = coeffs.magnitude_at(100.0, SAMPLE_RATE);
        assert!(mag_low > 0.9, "Low freq magnitude: {}", mag_low);

        // Should attenuate at high frequencies
        let mag_high = coeffs.magnitude_at(10000.0, SAMPLE_RATE);
        assert!(mag_high < 0.5, "High freq magnitude: {}", mag_high);
    }

    #[test]
    fn design_dc_blocker_test() {
        let coeffs = design_dc_blocker(SAMPLE_RATE);

        // Should block DC
        let mag_dc = coeffs.magnitude_at(1.0, SAMPLE_RATE);
        assert!(mag_dc < 0.1, "DC magnitude: {}", mag_dc);

        // Should pass audio
        let mag_audio = coeffs.magnitude_at(1000.0, SAMPLE_RATE);
        assert!(mag_audio > 0.99, "Audio magnitude: {}", mag_audio);
    }

    #[test]
    fn low_shelf_boost() {
        let coeffs = BiquadCoeffs::low_shelf(500.0, SAMPLE_RATE, 6.0, 1.0);

        // Should boost below shelf frequency
        let mag_low = coeffs.magnitude_at(100.0, SAMPLE_RATE);
        assert!(mag_low > 1.5, "Low freq magnitude: {}", mag_low);

        // Should be unity above shelf
        let mag_high = coeffs.magnitude_at(5000.0, SAMPLE_RATE);
        assert!(approx_eq(mag_high, 1.0, 0.1), "High freq magnitude: {}", mag_high);
    }

    #[test]
    fn high_shelf_boost() {
        let coeffs = BiquadCoeffs::high_shelf(2000.0, SAMPLE_RATE, 6.0, 1.0);

        // Should be unity below shelf frequency
        let mag_low = coeffs.magnitude_at(100.0, SAMPLE_RATE);
        assert!(approx_eq(mag_low, 1.0, 0.1), "Low freq magnitude: {}", mag_low);

        // Should boost above shelf
        let mag_high = coeffs.magnitude_at(10000.0, SAMPLE_RATE);
        assert!(mag_high > 1.5, "High freq magnitude: {}", mag_high);
    }
}
