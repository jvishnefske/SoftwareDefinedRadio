//! Modulation and Demodulation
//!
//! Provides modulation/demodulation algorithms for SSB, CW, AM, and FM.
//! Uses the Weaver method for SSB and standard techniques for other modes.

use super::filter::{BiquadCoeffs, BiquadFilter, DcBlocker};
use super::oscillator::{Nco, QuadratureOscillator};
#[cfg(feature = "embedded")]
use crate::types::Mode;
// F32Ext provides sqrt, sin, cos, atan2 for no_std; in std these are built-in
#[cfg(not(feature = "std"))]
use micromath::F32Ext;

/// IQ sample pair
#[derive(Clone, Copy, Debug, Default)]
pub struct IqSample {
    /// In-phase component
    pub i: f32,
    /// Quadrature component
    pub q: f32,
}

impl IqSample {
    /// Create a new IQ sample
    #[must_use]
    pub const fn new(i: f32, q: f32) -> Self {
        Self { i, q }
    }

    /// Get magnitude
    #[must_use]
    pub fn magnitude(&self) -> f32 {
        (self.i * self.i + self.q * self.q).sqrt()
    }

    /// Get phase in radians
    #[must_use]
    pub fn phase(&self) -> f32 {
        self.q.atan2(self.i)
    }

    /// Rotate by angle (radians)
    #[must_use]
    pub fn rotate(&self, angle: f32) -> Self {
        let (sin, cos) = (angle.sin(), angle.cos());
        Self {
            i: self.i * cos - self.q * sin,
            q: self.i * sin + self.q * cos,
        }
    }

    /// Complex multiply
    #[must_use]
    pub fn multiply(&self, other: Self) -> Self {
        Self {
            i: self.i * other.i - self.q * other.q,
            q: self.i * other.q + self.q * other.i,
        }
    }

    /// Complex conjugate
    #[must_use]
    pub const fn conjugate(&self) -> Self {
        Self {
            i: self.i,
            q: -self.q,
        }
    }

    /// Scale by a real factor
    #[must_use]
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            i: self.i * factor,
            q: self.q * factor,
        }
    }

    /// Add two IQ samples
    #[must_use]
    pub fn add(&self, other: Self) -> Self {
        Self {
            i: self.i + other.i,
            q: self.q + other.q,
        }
    }

    /// Subtract two IQ samples
    #[must_use]
    pub fn sub(&self, other: Self) -> Self {
        Self {
            i: self.i - other.i,
            q: self.q - other.q,
        }
    }

    /// Normalize to unit magnitude
    #[must_use]
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag > 1e-10 {
            Self {
                i: self.i / mag,
                q: self.q / mag,
            }
        } else {
            // Return zero for zero-magnitude inputs
            Self::default()
        }
    }
}

/// SSB demodulator using the phasing method
pub struct SsbDemodulator {
    /// I channel filter
    i_filter: BiquadFilter,
    /// Q channel filter
    q_filter: BiquadFilter,
    /// Hilbert transform filter for 90° phase shift
    hilbert: HilbertTransform,
    /// DC blocker
    dc_blocker: DcBlocker,
    /// Sideband selection (true = USB, false = LSB)
    usb: bool,
}

impl SsbDemodulator {
    /// Create a new SSB demodulator
    #[must_use]
    pub fn new(sample_rate: f32, bandwidth: f32) -> Self {
        // Bandpass filter for SSB (300-2700 Hz typical)
        let low_cutoff = 300.0 / sample_rate;
        let high_cutoff = bandwidth / sample_rate;
        let center = f32::midpoint(low_cutoff, high_cutoff);
        let q = center / (high_cutoff - low_cutoff);

        Self {
            i_filter: BiquadFilter::with_coeffs(BiquadCoeffs::bandpass(center, q)),
            q_filter: BiquadFilter::with_coeffs(BiquadCoeffs::bandpass(center, q)),
            hilbert: HilbertTransform::new(),
            dc_blocker: DcBlocker::default(),
            usb: true,
        }
    }

    /// Set sideband mode
    #[cfg(feature = "embedded")]
    pub fn set_mode(&mut self, mode: Mode) {
        self.usb = matches!(mode, Mode::Usb | Mode::Cw);
    }

    /// Set USB mode directly
    pub fn set_usb(&mut self, usb: bool) {
        self.usb = usb;
    }

    /// Process IQ sample to audio
    pub fn process(&mut self, iq: IqSample) -> f32 {
        // Filter I and Q channels
        let i = self.i_filter.process(iq.i);
        let q = self.q_filter.process(iq.q);

        // Apply Hilbert transform to I channel
        let i_shifted = self.hilbert.process(i);

        // Combine for sideband selection
        let audio = if self.usb {
            i_shifted + q
        } else {
            i_shifted - q
        };

        // Remove DC
        self.dc_blocker.process(audio)
    }

    /// Reset demodulator state
    pub fn reset(&mut self) {
        self.i_filter.reset();
        self.q_filter.reset();
        self.hilbert.reset();
        self.dc_blocker.reset();
    }
}

/// Hilbert transform for 90° phase shift
pub struct HilbertTransform {
    /// Filter delay line
    delay: [f32; 31],
    /// Current position
    pos: usize,
}

impl HilbertTransform {
    /// Hilbert filter coefficients (31-tap, odd samples only)
    const COEFFS: [f32; 16] = [
        0.0, 0.0636620, 0.0, 0.1061033, 0.0, 0.1591549, 0.0, 0.2122066,
        0.0, 0.3183099, 0.0, 0.6366198, 0.0, -0.6366198, 0.0, -0.3183099,
    ];

    /// Create a new Hilbert transform
    #[must_use]
    pub const fn new() -> Self {
        Self {
            delay: [0.0; 31],
            pos: 0,
        }
    }

    /// Process a sample
    pub fn process(&mut self, input: f32) -> f32 {
        self.delay[self.pos] = input;

        let mut output = 0.0;
        let mut idx = self.pos;

        // Apply odd-tap FIR
        for &coeff in &Self::COEFFS {
            if coeff != 0.0 {
                output += self.delay[idx] * coeff;
            }
            idx = if idx == 0 { 30 } else { idx - 1 };
        }

        self.pos = (self.pos + 1) % 31;
        output
    }

    /// Reset transform state
    pub fn reset(&mut self) {
        self.delay.fill(0.0);
        self.pos = 0;
    }
}

impl Default for HilbertTransform {
    fn default() -> Self {
        Self::new()
    }
}

/// AM demodulator using envelope detection
pub struct AmDemodulator {
    /// DC blocker for audio output
    dc_blocker: DcBlocker,
    /// Lowpass filter for envelope
    lpf: BiquadFilter,
}

impl AmDemodulator {
    /// Create a new AM demodulator
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let cutoff = 5000.0 / sample_rate;
        Self {
            dc_blocker: DcBlocker::default(),
            lpf: BiquadFilter::with_coeffs(BiquadCoeffs::lowpass(cutoff, 0.707)),
        }
    }

    /// Process IQ sample to audio
    pub fn process(&mut self, iq: IqSample) -> f32 {
        // Envelope detection
        let envelope = iq.magnitude();

        // Lowpass filter
        let filtered = self.lpf.process(envelope);

        // Remove DC
        self.dc_blocker.process(filtered)
    }

    /// Reset demodulator state
    pub fn reset(&mut self) {
        self.dc_blocker.reset();
        self.lpf.reset();
    }
}

/// FM demodulator using arctan differentiation
pub struct FmDemodulator {
    /// Previous IQ sample for differentiation
    prev_iq: IqSample,
    /// DC blocker
    dc_blocker: DcBlocker,
    /// Deemphasis filter
    deemph: BiquadFilter,
    /// Deviation scaling factor
    deviation_scale: f32,
}

impl FmDemodulator {
    /// Create a new FM demodulator
    #[must_use]
    pub fn new(sample_rate: f32, deviation_hz: f32) -> Self {
        // Deemphasis filter (75µs time constant for broadcast FM)
        let tau = 75e-6;
        let cutoff = 1.0 / (2.0 * core::f32::consts::PI * tau * sample_rate);

        Self {
            prev_iq: IqSample::default(),
            dc_blocker: DcBlocker::default(),
            deemph: BiquadFilter::with_coeffs(BiquadCoeffs::lowpass(cutoff, 0.707)),
            deviation_scale: sample_rate / (2.0 * core::f32::consts::PI * deviation_hz),
        }
    }

    /// Process IQ sample to audio
    pub fn process(&mut self, iq: IqSample) -> f32 {
        // Conjugate multiply with previous sample
        let product = iq.multiply(self.prev_iq.conjugate());
        self.prev_iq = iq;

        // Phase difference (FM discriminator)
        let phase_diff = product.q.atan2(product.i);

        // Scale and filter
        let audio = phase_diff * self.deviation_scale;
        let deemph = self.deemph.process(audio);
        self.dc_blocker.process(deemph)
    }

    /// Reset demodulator state
    pub fn reset(&mut self) {
        self.prev_iq = IqSample::default();
        self.dc_blocker.reset();
        self.deemph.reset();
    }
}

/// SSB modulator using the phasing method
pub struct SsbModulator {
    /// Hilbert transform
    hilbert: HilbertTransform,
    /// Carrier oscillator
    carrier: QuadratureOscillator,
    /// Audio bandpass filter
    audio_filter: BiquadFilter,
    /// USB mode
    usb: bool,
}

impl SsbModulator {
    /// Create a new SSB modulator
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        // Audio bandpass 300-2700 Hz
        let center = 1500.0 / sample_rate;
        let q = 1500.0 / 2400.0; // Bandwidth ratio

        Self {
            hilbert: HilbertTransform::new(),
            carrier: QuadratureOscillator::new(),
            audio_filter: BiquadFilter::with_coeffs(BiquadCoeffs::bandpass(center, q)),
            usb: true,
        }
    }

    /// Set sideband mode
    #[cfg(feature = "embedded")]
    pub fn set_mode(&mut self, mode: Mode) {
        self.usb = matches!(mode, Mode::Usb | Mode::Cw);
    }

    /// Set USB mode directly
    pub fn set_usb(&mut self, usb: bool) {
        self.usb = usb;
    }

    /// Set carrier frequency for up-conversion
    pub fn set_carrier(&mut self, freq_hz: f32, sample_rate: f32) {
        self.carrier.set_frequency(freq_hz, sample_rate);
    }

    /// Process audio sample to IQ
    pub fn process(&mut self, audio: f32) -> IqSample {
        // Filter audio
        let filtered = self.audio_filter.process(audio);

        // Generate I (original) and Q (Hilbert transformed)
        let i = filtered;
        let q = self.hilbert.process(filtered);

        // Create analytic signal (USB) or conjugate (LSB)
        if self.usb {
            IqSample::new(i, q)
        } else {
            IqSample::new(i, -q)
        }
    }

    /// Reset modulator state
    pub fn reset(&mut self) {
        self.hilbert.reset();
        self.carrier.reset();
        self.audio_filter.reset();
    }
}

/// AM modulator
pub struct AmModulator {
    /// Carrier oscillator
    carrier: Nco,
    /// Modulation depth (0.0 to 1.0)
    depth: f32,
}

impl AmModulator {
    /// Create a new AM modulator
    #[must_use]
    pub fn new(carrier_hz: f32, sample_rate: f32) -> Self {
        let mut carrier = Nco::new();
        carrier.set_frequency_f32(carrier_hz, sample_rate);
        Self {
            carrier,
            depth: 0.8,
        }
    }

    /// Set modulation depth
    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth.clamp(0.0, 1.0);
    }

    /// Process audio sample to modulated signal
    pub fn process(&mut self, audio: f32) -> f32 {
        let carrier = self.carrier.next();
        carrier * (1.0 + self.depth * audio)
    }
}

/// Complete demodulator supporting all modes
#[cfg(feature = "embedded")]
pub struct Demodulator {
    ssb: SsbDemodulator,
    am: AmDemodulator,
    fm: FmDemodulator,
    mode: Mode,
}

#[cfg(feature = "embedded")]
impl Demodulator {
    /// Create a new multi-mode demodulator
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            ssb: SsbDemodulator::new(sample_rate, 2700.0),
            am: AmDemodulator::new(sample_rate),
            fm: FmDemodulator::new(sample_rate, 5000.0),
            mode: Mode::Usb,
        }
    }

    /// Set operating mode
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.ssb.set_mode(mode);
    }

    /// Process IQ sample to audio
    pub fn process(&mut self, iq: IqSample) -> f32 {
        match self.mode {
            Mode::Lsb | Mode::Usb => self.ssb.process(iq),
            Mode::Cw | Mode::CwR => {
                self.ssb.set_mode(if matches!(self.mode, Mode::Cw) {
                    Mode::Usb
                } else {
                    Mode::Lsb
                });
                self.ssb.process(iq)
            }
            Mode::Am => self.am.process(iq),
            Mode::Fm => self.fm.process(iq),
        }
    }

    /// Reset all demodulators
    pub fn reset(&mut self) {
        self.ssb.reset();
        self.am.reset();
        self.fm.reset();
    }
}
