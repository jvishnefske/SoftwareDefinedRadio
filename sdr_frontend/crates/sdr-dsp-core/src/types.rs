//! Core types for SDR DSP processing.

#[allow(unused_imports)]
use micromath::F32Ext;

/// IQ sample pair representing complex baseband signal.
///
/// In-phase (I) and Quadrature (Q) components represent the real and imaginary
/// parts of a complex signal. This is the fundamental data type for SDR processing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IqSample {
    /// In-phase component (real part)
    pub i: f32,
    /// Quadrature component (imaginary part)
    pub q: f32,
}

impl IqSample {
    /// Create a new IQ sample.
    #[must_use]
    #[inline]
    pub const fn new(i: f32, q: f32) -> Self {
        Self { i, q }
    }

    /// Create an IQ sample from a real value (Q = 0).
    #[must_use]
    #[inline]
    pub const fn from_real(i: f32) -> Self {
        Self { i, q: 0.0 }
    }

    /// Zero sample.
    pub const ZERO: Self = Self { i: 0.0, q: 0.0 };

    /// Calculate magnitude (absolute value).
    #[must_use]
    #[inline]
    pub fn magnitude(&self) -> f32 {
        (self.i * self.i + self.q * self.q).sqrt()
    }

    /// Calculate magnitude squared (avoids sqrt for comparisons).
    #[must_use]
    #[inline]
    pub fn magnitude_squared(&self) -> f32 {
        self.i * self.i + self.q * self.q
    }

    /// Calculate phase angle in radians (-π to π).
    #[must_use]
    #[inline]
    pub fn phase(&self) -> f32 {
        self.q.atan2(self.i)
    }

    /// Rotate sample by angle in radians.
    #[must_use]
    #[inline]
    pub fn rotate(&self, angle: f32) -> Self {
        let (sin, cos) = (angle.sin(), angle.cos());
        Self {
            i: self.i * cos - self.q * sin,
            q: self.i * sin + self.q * cos,
        }
    }

    /// Complex multiply with another sample.
    #[must_use]
    #[inline]
    pub fn multiply(&self, other: Self) -> Self {
        Self {
            i: self.i * other.i - self.q * other.q,
            q: self.i * other.q + self.q * other.i,
        }
    }

    /// Complex conjugate (negate Q component).
    #[must_use]
    #[inline]
    pub const fn conjugate(&self) -> Self {
        Self {
            i: self.i,
            q: -self.q,
        }
    }

    /// Scale by a real factor.
    #[must_use]
    #[inline]
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            i: self.i * factor,
            q: self.q * factor,
        }
    }

    /// Add two IQ samples.
    #[must_use]
    #[inline]
    pub fn add(&self, other: Self) -> Self {
        Self {
            i: self.i + other.i,
            q: self.q + other.q,
        }
    }

    /// Subtract two IQ samples.
    #[must_use]
    #[inline]
    pub fn sub(&self, other: Self) -> Self {
        Self {
            i: self.i - other.i,
            q: self.q - other.q,
        }
    }

    /// Normalize to unit magnitude.
    ///
    /// Returns zero if magnitude is too small to normalize.
    #[must_use]
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag < 1e-10 {
            Self::ZERO
        } else {
            self.scale(1.0 / mag)
        }
    }
}

impl core::ops::Add for IqSample {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::add(&self, other)
    }
}

impl core::ops::Sub for IqSample {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::sub(&self, other)
    }
}

impl core::ops::Mul<f32> for IqSample {
    type Output = Self;

    fn mul(self, factor: f32) -> Self {
        self.scale(factor)
    }
}

impl core::ops::Mul for IqSample {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        self.multiply(other)
    }
}

/// Signal quality metrics for display and squelch.
#[derive(Clone, Copy, Debug, Default)]
pub struct SignalMetrics {
    /// Signal-to-noise ratio in dB.
    pub snr_db: f32,
    /// Intermodulation distortion in dB (for PSK).
    pub imd_db: f32,
    /// AFC frequency offset in Hz.
    pub afc_offset_hz: f32,
    /// Symbol timing error (normalized).
    pub timing_error: f32,
    /// Whether signal is above squelch threshold.
    pub squelch_open: bool,
    /// Decode confidence (0.0 to 1.0).
    pub confidence: f32,
}

impl SignalMetrics {
    /// Create new metrics with default values.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            snr_db: -30.0,
            imd_db: -30.0,
            afc_offset_hz: 0.0,
            timing_error: 0.0,
            squelch_open: false,
            confidence: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iq_magnitude() {
        let sample = IqSample::new(3.0, 4.0);
        assert!((sample.magnitude() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_iq_phase() {
        let sample = IqSample::new(1.0, 0.0);
        assert!(sample.phase().abs() < 1e-6);

        let sample = IqSample::new(0.0, 1.0);
        assert!((sample.phase() - core::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn test_iq_rotate() {
        let sample = IqSample::new(1.0, 0.0);
        let rotated = sample.rotate(core::f32::consts::FRAC_PI_2);
        assert!(rotated.i.abs() < 1e-6);
        assert!((rotated.q - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_iq_multiply() {
        let a = IqSample::new(1.0, 2.0);
        let b = IqSample::new(3.0, 4.0);
        let result = a.multiply(b);
        // (1+2j)(3+4j) = 3 + 4j + 6j + 8j² = 3 + 10j - 8 = -5 + 10j
        assert!((result.i - (-5.0)).abs() < 1e-6);
        assert!((result.q - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_iq_normalize() {
        let sample = IqSample::new(3.0, 4.0);
        let normalized = sample.normalize();
        assert!((normalized.magnitude() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_iq_zero_normalize() {
        let sample = IqSample::new(0.0, 0.0);
        let normalized = sample.normalize();
        assert!(normalized.i.abs() < 1e-6);
        assert!(normalized.q.abs() < 1e-6);
    }
}
