//! WASM bindings for SDR DSP processing.
//!
//! This crate provides WebAssembly bindings for the DSP modules,
//! designed to run in an AudioWorklet for real-time audio processing.

use sdr_dsp_core::{Agc, AgcConfig, Biquad, DcBlocker, FftSpectrum, IqSample, Nco, SMeter};
use wasm_bindgen::prelude::*;

/// Audio buffer size (matches AudioWorklet quantum).
pub const BUFFER_SIZE: usize = 128;

/// Spectrum FFT size.
pub const SPECTRUM_SIZE: usize = 512;

/// DSP processor for AudioWorklet integration.
///
/// Handles IQ demodulation, filtering, AGC, and spectrum analysis.
#[wasm_bindgen]
pub struct DspProcessor {
    // Input/output buffers (interleaved I,Q for input)
    input_buffer: [f32; BUFFER_SIZE * 2],
    output_buffer: [f32; BUFFER_SIZE],
    spectrum_buffer: [f32; SPECTRUM_SIZE],

    // DSP components
    dc_blocker_i: DcBlocker,
    dc_blocker_q: DcBlocker,
    nco: Nco,
    audio_filter: Biquad,
    agc: Agc,
    smeter: SMeter,
    spectrum: FftSpectrum,

    // Configuration
    sample_rate: f32,
    mode: u8,         // 0=LSB, 1=USB, 2=CW, 3=AM, 4=FM
    freq_offset: f32, // Audio frequency offset in Hz

    // State
    frame_count: u32,
    smeter_value: f32,
}

#[wasm_bindgen]
impl DspProcessor {
    /// Create a new DSP processor.
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f32) -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        let agc_config = AgcConfig::medium();

        Self {
            input_buffer: [0.0; BUFFER_SIZE * 2],
            output_buffer: [0.0; BUFFER_SIZE],
            spectrum_buffer: [0.0; SPECTRUM_SIZE],
            dc_blocker_i: DcBlocker::default(),
            dc_blocker_q: DcBlocker::default(),
            nco: Nco::new(sample_rate, 0.0),
            audio_filter: Biquad::lowpass(sample_rate, 2700.0, 0.707),
            agc: Agc::new(sample_rate, agc_config),
            smeter: SMeter::new(sample_rate, 100.0),
            spectrum: FftSpectrum::new(SPECTRUM_SIZE),
            sample_rate,
            mode: 1, // USB default
            freq_offset: 1500.0,
            frame_count: 0,
            smeter_value: 0.0,
        }
    }

    /// Get pointer to input buffer for WASM memory access.
    #[wasm_bindgen]
    pub fn get_input_buffer_ptr(&mut self) -> *mut f32 {
        self.input_buffer.as_mut_ptr()
    }

    /// Get pointer to output buffer for WASM memory access.
    #[wasm_bindgen]
    pub fn get_output_buffer_ptr(&self) -> *const f32 {
        self.output_buffer.as_ptr()
    }

    /// Get pointer to spectrum buffer for WASM memory access.
    #[wasm_bindgen]
    pub fn get_spectrum_buffer_ptr(&self) -> *const f32 {
        self.spectrum_buffer.as_ptr()
    }

    /// Process audio samples.
    ///
    /// Input: interleaved I/Q samples (I0, Q0, I1, Q1, ...)
    /// Output: mono audio samples
    #[wasm_bindgen]
    pub fn process(&mut self, num_samples: usize) {
        let samples = num_samples.min(BUFFER_SIZE);

        for idx in 0..samples {
            // Extract I/Q from interleaved buffer
            let raw_i = self.input_buffer[idx * 2];
            let raw_q = self.input_buffer[idx * 2 + 1];

            // DC blocking
            let i_sample = self.dc_blocker_i.process(raw_i);
            let q_sample = self.dc_blocker_q.process(raw_q);

            let iq = IqSample::new(i_sample, q_sample);

            // Mix to audio frequency
            let mixed = self.nco.mix(iq);

            // Demodulate based on mode
            let audio = match self.mode {
                0 => self.demod_lsb(mixed),
                1 => self.demod_usb(mixed),
                2 => self.demod_cw(mixed),
                3 => self.demod_am(mixed),
                4 => self.demod_fm(mixed),
                _ => self.demod_usb(mixed),
            };

            // Apply audio filter
            let filtered = self.audio_filter.process(audio);

            // AGC
            let output = self.agc.process(filtered);

            // Update S-meter
            self.smeter.update(iq.magnitude());

            // Store output
            self.output_buffer[idx] = output;

            // Feed spectrum analyzer
            self.spectrum.push(iq.magnitude());
        }

        // Update S-meter reading
        self.smeter_value = self.smeter.value();

        // Compute spectrum if buffer full
        if self.spectrum.is_ready() {
            self.spectrum.compute(&mut self.spectrum_buffer);
        }

        self.frame_count += 1;
    }

    /// LSB demodulation (I - Q shifted).
    fn demod_lsb(&self, iq: IqSample) -> f32 {
        // Simple LSB: take I component (after mixing)
        iq.i - iq.q
    }

    /// USB demodulation (I + Q shifted).
    fn demod_usb(&self, iq: IqSample) -> f32 {
        // Simple USB: I + Q
        iq.i + iq.q
    }

    /// CW demodulation (beat frequency oscillator).
    fn demod_cw(&self, iq: IqSample) -> f32 {
        // CW is essentially USB with narrow filter
        iq.i + iq.q
    }

    /// AM demodulation (envelope detection).
    fn demod_am(&self, iq: IqSample) -> f32 {
        iq.magnitude()
    }

    /// FM demodulation (phase derivative).
    fn demod_fm(&self, iq: IqSample) -> f32 {
        // Simplified FM demod using phase
        iq.phase()
    }

    /// Set operating mode.
    #[wasm_bindgen]
    pub fn set_mode(&mut self, mode: u8) {
        self.mode = mode;

        // Adjust filter bandwidth based on mode
        let bandwidth = match mode {
            0 | 1 => 2700.0, // SSB
            2 => 500.0,      // CW
            3 => 6000.0,     // AM
            4 => 15000.0,    // FM
            _ => 2700.0,
        };

        self.audio_filter = Biquad::lowpass(self.sample_rate, bandwidth, 0.707);
    }

    /// Set frequency offset for mixing.
    #[wasm_bindgen]
    pub fn set_frequency_offset(&mut self, offset_hz: f32) {
        self.freq_offset = offset_hz;
        self.nco.set_frequency(offset_hz);
    }

    /// Set filter bandwidth in Hz.
    #[wasm_bindgen]
    pub fn set_filter_bandwidth(&mut self, bandwidth_hz: f32) {
        self.audio_filter = Biquad::lowpass(self.sample_rate, bandwidth_hz, 0.707);
    }

    /// Set AGC parameters.
    #[wasm_bindgen]
    pub fn set_agc(&mut self, attack_ms: f32, decay_ms: f32, hang_ms: f32) {
        let config = AgcConfig {
            attack_ms,
            decay_ms,
            hang_ms,
            ..AgcConfig::default()
        };
        self.agc.set_config(config);
    }

    /// Get current S-meter value (0.0 to ~1.5).
    #[wasm_bindgen]
    pub fn get_smeter(&self) -> f32 {
        self.smeter_value
    }

    /// Get current frame count.
    #[wasm_bindgen]
    pub fn get_frame_count(&self) -> u32 {
        self.frame_count
    }

    /// Reset processor state.
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.dc_blocker_i.reset();
        self.dc_blocker_q.reset();
        self.nco.reset();
        self.agc.reset();
        self.smeter.reset();
        self.spectrum.reset();
        self.frame_count = 0;
    }
}

/// Create a new DSP processor (factory function).
#[wasm_bindgen]
pub fn create_processor(sample_rate: f32) -> DspProcessor {
    DspProcessor::new(sample_rate)
}
