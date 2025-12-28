//! PSK31 decoder implementation.

use crate::varicode::{VaricodeDecoder, VaricodeError};
#[allow(unused_imports)]
use micromath::F32Ext;
use sdr_dsp_core::{Biquad, CostasLoop, IqSample, Nco, SignalMetrics};

/// PSK31 decoder configuration.
#[derive(Clone, Debug)]
pub struct Psk31DecoderConfig {
    /// Sample rate in Hz
    pub sample_rate: f32,
    /// Center frequency offset in Hz
    pub center_freq_hz: f32,
    /// Enable AFC (Automatic Frequency Control)
    pub afc_enabled: bool,
    /// AFC bandwidth in Hz
    pub afc_bandwidth: f32,
    /// Squelch threshold (0.0 to 1.0)
    pub squelch_threshold: f32,
    /// QPSK mode (false = BPSK)
    pub qpsk_mode: bool,
}

impl Default for Psk31DecoderConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000.0,
            center_freq_hz: 1500.0,
            afc_enabled: true,
            afc_bandwidth: 50.0,
            squelch_threshold: 0.3,
            qpsk_mode: false,
        }
    }
}

/// PSK31 decoder state.
pub struct Psk31Decoder {
    config: Psk31DecoderConfig,

    // Carrier tracking
    nco: Nco,
    costas: CostasLoop,

    // Symbol timing
    samples_per_symbol: f32,
    sample_count: f32,
    timing_error: f32,

    // Matched filter (raised cosine)
    matched_filter: Biquad,

    // Symbol history for timing recovery
    prev_sample: IqSample,
    prev_prev_sample: IqSample,

    // Differential decode
    prev_phase: f32,

    // Varicode decode
    varicode: VaricodeDecoder,

    // Signal quality
    signal_power: f32,
    noise_power: f32,
    imd_peak: f32,
    imd_avg: f32,

    // AFC state
    afc_offset: f32,
}

/// PSK31 decode error.
#[derive(Clone, Copy, Debug)]
pub enum Psk31Error {
    /// Varicode decode error
    Varicode(VaricodeError),
    /// Signal below squelch
    BelowSquelch,
}

impl From<VaricodeError> for Psk31Error {
    fn from(e: VaricodeError) -> Self {
        Psk31Error::Varicode(e)
    }
}

impl Psk31Decoder {
    /// Create a new PSK31 decoder.
    #[must_use]
    pub fn new(config: Psk31DecoderConfig) -> Self {
        let sample_rate = config.sample_rate;
        let center_freq = config.center_freq_hz;

        // PSK31 baud rate
        const BAUD_RATE: f32 = 31.25;
        let samples_per_symbol = sample_rate / BAUD_RATE;

        // Matched filter bandwidth (approximately baud rate)
        let matched_filter = Biquad::lowpass(sample_rate, BAUD_RATE * 1.5, 0.707);

        Self {
            config: config.clone(),
            nco: Nco::new(sample_rate, center_freq),
            costas: CostasLoop::new(sample_rate, 0.0, config.afc_bandwidth),
            samples_per_symbol,
            sample_count: 0.0,
            timing_error: 0.0,
            matched_filter,
            prev_sample: IqSample::ZERO,
            prev_prev_sample: IqSample::ZERO,
            prev_phase: 0.0,
            varicode: VaricodeDecoder::new(),
            signal_power: 0.0,
            noise_power: 0.001,
            imd_peak: 0.0,
            imd_avg: 0.0,
            afc_offset: 0.0,
        }
    }

    /// Process a single IQ sample.
    ///
    /// Returns decoded character if available.
    pub fn process(&mut self, iq: IqSample) -> Result<Option<char>, Psk31Error> {
        // 1. Mix to baseband
        let baseband = self.nco.mix(iq);

        // 2. Apply matched filter
        let filtered = self.matched_filter.process_iq(baseband);

        // 3. Carrier tracking via Costas loop
        let (tracked, _phase_error) = self.costas.process(filtered);

        // 4. Update AFC
        if self.config.afc_enabled {
            self.afc_offset = self.costas.frequency_offset();
        }

        // 5. Update signal/noise power estimates
        let power = tracked.magnitude_squared();
        self.signal_power = 0.99 * self.signal_power + 0.01 * power;

        // 6. Symbol timing recovery (Gardner algorithm)
        self.sample_count += 1.0;

        if self.sample_count >= self.samples_per_symbol {
            self.sample_count -= self.samples_per_symbol;

            // Gardner timing error detector
            let mid_sample = self.prev_sample;
            let timing_error = (tracked.i - self.prev_prev_sample.i) * mid_sample.i
                + (tracked.q - self.prev_prev_sample.q) * mid_sample.q;

            // Adjust timing
            self.timing_error = 0.9 * self.timing_error + 0.1 * timing_error;
            self.sample_count += 0.01 * self.timing_error;

            // 7. Differential decode (BPSK)
            let phase = tracked.phase();
            let phase_diff = self.wrap_phase(phase - self.prev_phase);
            self.prev_phase = phase;

            // Decision: phase change near 0 = 1, near π = 0
            let bit = phase_diff.abs() < core::f32::consts::FRAC_PI_2;

            // 8. Update IMD estimate
            let mag = tracked.magnitude();
            if mag > self.imd_peak {
                self.imd_peak = mag;
            }
            self.imd_avg = 0.99 * self.imd_avg + 0.01 * mag;

            // 9. Check squelch
            let snr = self.signal_power / self.noise_power.max(0.0001);
            if snr < self.config.squelch_threshold {
                return Err(Psk31Error::BelowSquelch);
            }

            // 10. Varicode decode
            match self.varicode.push_bit(bit) {
                Ok(Some(ch)) => {
                    // Reset IMD on character decode
                    self.imd_peak = 0.0;
                    return Ok(Some(ch));
                }
                Ok(None) => {}
                Err(e) => return Err(Psk31Error::Varicode(e)),
            }
        }

        // Update history
        self.prev_prev_sample = self.prev_sample;
        self.prev_sample = tracked;

        Ok(None)
    }

    /// Wrap phase to [-π, π].
    fn wrap_phase(&self, phase: f32) -> f32 {
        let mut p = phase;
        while p > core::f32::consts::PI {
            p -= 2.0 * core::f32::consts::PI;
        }
        while p < -core::f32::consts::PI {
            p += 2.0 * core::f32::consts::PI;
        }
        p
    }

    /// Get signal quality metrics.
    #[must_use]
    pub fn metrics(&self) -> SignalMetrics {
        let snr_db = 10.0
            * (self.signal_power / self.noise_power.max(0.0001))
                .max(0.001)
                .log10();

        let imd_db = if self.imd_peak > 0.0 {
            20.0 * (self.imd_avg / self.imd_peak.max(0.001)).max(0.001).log10()
        } else {
            -30.0
        };

        SignalMetrics {
            snr_db,
            imd_db,
            afc_offset_hz: self.afc_offset,
            timing_error: self.timing_error,
            squelch_open: self.signal_power > self.config.squelch_threshold * self.noise_power,
            confidence: (snr_db / 20.0).clamp(0.0, 1.0),
        }
    }

    /// Get AFC frequency offset in Hz.
    #[must_use]
    pub fn afc_offset(&self) -> f32 {
        self.afc_offset
    }

    /// Set center frequency.
    pub fn set_frequency(&mut self, freq_hz: f32) {
        self.nco.set_frequency(freq_hz);
    }

    /// Reset decoder state.
    pub fn reset(&mut self) {
        self.nco.reset();
        self.costas.reset();
        self.varicode.reset();
        self.sample_count = 0.0;
        self.timing_error = 0.0;
        self.prev_sample = IqSample::ZERO;
        self.prev_prev_sample = IqSample::ZERO;
        self.prev_phase = 0.0;
        self.signal_power = 0.0;
        self.imd_peak = 0.0;
        self.imd_avg = 0.0;
    }
}
