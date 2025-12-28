//! Audio DSP Processing Chain
//!
//! Integrates filters, AGC, and other DSP elements into a complete
//! receive audio processing pipeline for each modulation mode.

use super::agc::{Agc, AgcConfig, SMeter};
use super::filter_design::{
    design_am_filter, design_cw_filter, design_dc_blocker, design_ssb_filter,
    design_deemphasis_filter, AmBandwidth, Biquad, CwBandwidth, SsbBandwidth,
};

/// Sample rate used by the audio chain
pub const AUDIO_SAMPLE_RATE: f32 = 48000.0;

/// Complete audio processing chain for receive
#[derive(Clone)]
pub struct AudioChain {
    /// Mode-specific filter(s)
    filter_stage: FilterStage,
    /// DC blocking filter
    dc_blocker: Biquad,
    /// AGC processor
    agc: Agc,
    /// S-meter
    smeter: SMeter,
    /// Volume (0.0 to 1.0)
    volume: f32,
    /// Muted state
    muted: bool,
}

/// Filter configuration for different modes
#[derive(Clone)]
#[allow(missing_docs)]
pub enum FilterStage {
    /// CW: single bandpass filter
    Cw {
        /// Bandpass filter centered on CW tone
        bandpass: Biquad,
        /// Center frequency in Hz
        center_freq: f32,
        /// Filter bandwidth
        bandwidth: CwBandwidth,
    },
    /// SSB: highpass + lowpass for voice audio
    Ssb {
        /// High-pass filter for low-frequency rejection
        highpass: Biquad,
        /// Low-pass filter for high-frequency limit
        lowpass: Biquad,
        /// Overall SSB bandwidth
        bandwidth: SsbBandwidth,
    },
    /// AM: lowpass only
    Am {
        /// Low-pass filter for audio bandwidth
        lowpass: Biquad,
        /// AM bandwidth setting
        bandwidth: AmBandwidth,
    },
    /// FM: de-emphasis filter for broadcast audio
    Fm {
        /// De-emphasis filter (75µs or 50µs)
        deemphasis: Biquad,
    },
    /// Bypass: no filtering
    Bypass,
}

impl AudioChain {
    /// Create a new audio chain for CW mode
    #[must_use]
    pub fn new_cw(center_freq: f32, bandwidth: CwBandwidth) -> Self {
        let coeffs = design_cw_filter(center_freq, bandwidth, AUDIO_SAMPLE_RATE);
        Self {
            filter_stage: FilterStage::Cw {
                bandpass: Biquad::new(coeffs),
                center_freq,
                bandwidth,
            },
            dc_blocker: Biquad::new(design_dc_blocker(AUDIO_SAMPLE_RATE)),
            agc: Agc::new(AgcConfig::from_ms(AUDIO_SAMPLE_RATE as u32, 5, 500)),
            smeter: SMeter::new(),
            volume: 0.5,
            muted: false,
        }
    }

    /// Create a new audio chain for SSB mode
    #[must_use]
    pub fn new_ssb(bandwidth: SsbBandwidth) -> Self {
        let (hpf_coeffs, lpf_coeffs) = design_ssb_filter(bandwidth, AUDIO_SAMPLE_RATE);
        Self {
            filter_stage: FilterStage::Ssb {
                highpass: Biquad::new(hpf_coeffs),
                lowpass: Biquad::new(lpf_coeffs),
                bandwidth,
            },
            dc_blocker: Biquad::new(design_dc_blocker(AUDIO_SAMPLE_RATE)),
            agc: Agc::new(AgcConfig::from_ms(AUDIO_SAMPLE_RATE as u32, 10, 500)),
            smeter: SMeter::new(),
            volume: 0.5,
            muted: false,
        }
    }

    /// Create a new audio chain for AM mode
    #[must_use]
    pub fn new_am(bandwidth: AmBandwidth) -> Self {
        let coeffs = design_am_filter(bandwidth, AUDIO_SAMPLE_RATE);
        Self {
            filter_stage: FilterStage::Am {
                lowpass: Biquad::new(coeffs),
                bandwidth,
            },
            dc_blocker: Biquad::new(design_dc_blocker(AUDIO_SAMPLE_RATE)),
            agc: Agc::new(AgcConfig::from_ms(AUDIO_SAMPLE_RATE as u32, 20, 1000)),
            smeter: SMeter::new(),
            volume: 0.5,
            muted: false,
        }
    }

    /// Create a new audio chain for FM mode with de-emphasis
    #[must_use]
    pub fn new_fm() -> Self {
        // 75µs de-emphasis (US/Japan standard)
        let coeffs = design_deemphasis_filter(75.0, AUDIO_SAMPLE_RATE);
        Self {
            filter_stage: FilterStage::Fm {
                deemphasis: Biquad::new(coeffs),
            },
            dc_blocker: Biquad::new(design_dc_blocker(AUDIO_SAMPLE_RATE)),
            agc: Agc::new(AgcConfig::from_ms(AUDIO_SAMPLE_RATE as u32, 10, 200)),
            smeter: SMeter::new(),
            volume: 0.5,
            muted: false,
        }
    }

    /// Create a bypass chain (no filtering)
    #[must_use]
    pub fn new_bypass() -> Self {
        Self {
            filter_stage: FilterStage::Bypass,
            dc_blocker: Biquad::new(design_dc_blocker(AUDIO_SAMPLE_RATE)),
            agc: Agc::new(AgcConfig::default()),
            smeter: SMeter::new(),
            volume: 0.5,
            muted: false,
        }
    }

    /// Process a single sample through the chain
    pub fn process(&mut self, input: f32) -> f32 {
        if self.muted {
            // Still update S-meter even when muted
            self.smeter.update_from_level(input.abs());
            return 0.0;
        }

        // Stage 1: DC blocking
        let sample = self.dc_blocker.process(input);

        // Stage 2: Mode-specific filtering
        let sample = match &mut self.filter_stage {
            FilterStage::Cw { bandpass, .. } => bandpass.process(sample),
            FilterStage::Ssb {
                highpass, lowpass, ..
            } => {
                let hp_out = highpass.process(sample);
                lowpass.process(hp_out)
            }
            FilterStage::Am { lowpass, .. } => lowpass.process(sample),
            FilterStage::Fm { deemphasis } => deemphasis.process(sample),
            FilterStage::Bypass => sample,
        };

        // Stage 3: AGC
        let sample = self.agc.process(sample);

        // Update S-meter from AGC
        self.smeter.update_from_agc(&self.agc);

        // Stage 4: Volume control
        sample * self.volume
    }

    /// Process a block of samples in-place
    pub fn process_block(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Get current S-meter reading
    #[must_use]
    pub fn smeter(&self) -> &SMeter {
        &self.smeter
    }

    /// Get current AGC gain in dB
    #[must_use]
    pub fn agc_gain_db(&self) -> f32 {
        self.agc.gain_db()
    }

    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Get current volume
    #[must_use]
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Mute/unmute audio output
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// Check if muted
    #[must_use]
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Update CW filter center frequency
    pub fn set_cw_frequency(&mut self, center_freq: f32) {
        if let FilterStage::Cw {
            bandpass,
            center_freq: freq,
            bandwidth,
        } = &mut self.filter_stage
        {
            *freq = center_freq;
            let coeffs = design_cw_filter(center_freq, *bandwidth, AUDIO_SAMPLE_RATE);
            *bandpass = Biquad::new(coeffs);
        }
    }

    /// Update CW bandwidth
    pub fn set_cw_bandwidth(&mut self, new_bandwidth: CwBandwidth) {
        if let FilterStage::Cw {
            bandpass,
            center_freq,
            bandwidth,
        } = &mut self.filter_stage
        {
            *bandwidth = new_bandwidth;
            let coeffs = design_cw_filter(*center_freq, new_bandwidth, AUDIO_SAMPLE_RATE);
            *bandpass = Biquad::new(coeffs);
        }
    }

    /// Update SSB bandwidth
    pub fn set_ssb_bandwidth(&mut self, new_bandwidth: SsbBandwidth) {
        if let FilterStage::Ssb {
            highpass,
            lowpass,
            bandwidth,
        } = &mut self.filter_stage
        {
            *bandwidth = new_bandwidth;
            let (hpf_coeffs, lpf_coeffs) = design_ssb_filter(new_bandwidth, AUDIO_SAMPLE_RATE);
            *highpass = Biquad::new(hpf_coeffs);
            *lowpass = Biquad::new(lpf_coeffs);
        }
    }

    /// Update AM bandwidth
    pub fn set_am_bandwidth(&mut self, new_bandwidth: AmBandwidth) {
        if let FilterStage::Am { lowpass, bandwidth } = &mut self.filter_stage {
            *bandwidth = new_bandwidth;
            let coeffs = design_am_filter(new_bandwidth, AUDIO_SAMPLE_RATE);
            *lowpass = Biquad::new(coeffs);
        }
    }

    /// Reset all internal state (filters, AGC)
    pub fn reset(&mut self) {
        self.dc_blocker.reset();
        match &mut self.filter_stage {
            FilterStage::Cw { bandpass, .. } => bandpass.reset(),
            FilterStage::Ssb {
                highpass, lowpass, ..
            } => {
                highpass.reset();
                lowpass.reset();
            }
            FilterStage::Am { lowpass, .. } => lowpass.reset(),
            FilterStage::Fm { deemphasis } => deemphasis.reset(),
            FilterStage::Bypass => {}
        }
        self.agc.reset();
    }

    /// Configure AGC parameters
    pub fn set_agc_config(&mut self, config: AgcConfig) {
        self.agc.set_config(config);
    }

    /// Get the current filter stage type name
    #[must_use]
    pub fn mode_name(&self) -> &'static str {
        match &self.filter_stage {
            FilterStage::Cw { .. } => "CW",
            FilterStage::Ssb { .. } => "SSB",
            FilterStage::Am { .. } => "AM",
            FilterStage::Fm { .. } => "FM",
            FilterStage::Bypass => "Bypass",
        }
    }
}

impl Default for AudioChain {
    fn default() -> Self {
        Self::new_ssb(SsbBandwidth::Standard)
    }
}

/// Notch filter for removing interference
#[derive(Clone)]
pub struct NotchFilter {
    filter: Biquad,
    frequency: f32,
    enabled: bool,
}

impl NotchFilter {
    /// Create a new notch filter at the specified frequency
    #[must_use]
    pub fn new(frequency: f32) -> Self {
        use super::filter_design::BiquadCoeffs;
        let coeffs = BiquadCoeffs::notch(frequency, AUDIO_SAMPLE_RATE, 10.0);
        Self {
            filter: Biquad::new(coeffs),
            frequency,
            enabled: true,
        }
    }

    /// Process a sample
    pub fn process(&mut self, input: f32) -> f32 {
        if self.enabled {
            self.filter.process(input)
        } else {
            input
        }
    }

    /// Set notch frequency
    pub fn set_frequency(&mut self, frequency: f32) {
        use super::filter_design::BiquadCoeffs;
        self.frequency = frequency;
        let coeffs = BiquadCoeffs::notch(frequency, AUDIO_SAMPLE_RATE, 10.0);
        self.filter = Biquad::new(coeffs);
    }

    /// Enable/disable the notch
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get notch frequency
    #[must_use]
    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    /// Check if enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for NotchFilter {
    fn default() -> Self {
        Self::new(1000.0)
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn audio_chain_cw_creation() {
        let chain = AudioChain::new_cw(700.0, CwBandwidth::Hz400);
        assert_eq!(chain.mode_name(), "CW");
        assert!(!chain.is_muted());
    }

    #[test]
    fn audio_chain_ssb_creation() {
        let chain = AudioChain::new_ssb(SsbBandwidth::Standard);
        assert_eq!(chain.mode_name(), "SSB");
    }

    #[test]
    fn audio_chain_am_creation() {
        let chain = AudioChain::new_am(AmBandwidth::Standard);
        assert_eq!(chain.mode_name(), "AM");
    }

    #[test]
    fn audio_chain_fm_creation() {
        let chain = AudioChain::new_fm();
        assert_eq!(chain.mode_name(), "FM");
    }

    #[test]
    fn audio_chain_bypass_creation() {
        let chain = AudioChain::new_bypass();
        assert_eq!(chain.mode_name(), "Bypass");
    }

    #[test]
    fn audio_chain_default() {
        let chain = AudioChain::default();
        assert_eq!(chain.mode_name(), "SSB");
    }

    #[test]
    fn audio_chain_process_finite() {
        let mut chain = AudioChain::new_ssb(SsbBandwidth::Standard);
        for level in [0.0, 0.1, 0.5, 1.0] {
            let output = chain.process(level);
            assert!(output.is_finite(), "Output should be finite for {}", level);
        }
    }

    #[test]
    fn audio_chain_volume_control() {
        let mut chain = AudioChain::new_bypass();
        chain.set_volume(1.0);

        // Process signal
        for _ in 0..100 {
            chain.process(0.5);
        }

        // Reduce volume
        chain.set_volume(0.5);
        let output = chain.process(0.5);

        assert!(output < 0.5, "Reduced volume should lower output");
    }

    #[test]
    fn audio_chain_volume_clamp() {
        let mut chain = AudioChain::new_bypass();

        chain.set_volume(1.5);
        assert_eq!(chain.volume(), 1.0);

        chain.set_volume(-0.5);
        assert_eq!(chain.volume(), 0.0);
    }

    #[test]
    fn audio_chain_mute() {
        let mut chain = AudioChain::new_bypass();

        chain.set_muted(true);
        assert!(chain.is_muted());

        let output = chain.process(0.5);
        assert_eq!(output, 0.0, "Muted chain should output zero");

        chain.set_muted(false);
        assert!(!chain.is_muted());
    }

    #[test]
    fn audio_chain_smeter_updates() {
        let mut chain = AudioChain::new_ssb(SsbBandwidth::Standard);

        // Process signal
        for _ in 0..1000 {
            chain.process(0.3);
        }

        let s = chain.smeter().s_units();
        assert!(s <= 9, "S-meter should read valid value, got S{}", s);
    }

    #[test]
    fn audio_chain_agc_gain() {
        let mut chain = AudioChain::new_ssb(SsbBandwidth::Standard);

        // Process weak signal
        for _ in 0..1000 {
            chain.process(0.01);
        }

        let gain_db = chain.agc_gain_db();
        assert!(gain_db.is_finite());
    }

    #[test]
    fn audio_chain_process_block() {
        let mut chain = AudioChain::new_ssb(SsbBandwidth::Standard);
        let mut samples = [0.1, 0.2, 0.3, 0.4, 0.5];
        chain.process_block(&mut samples);

        for &s in &samples {
            assert!(s.is_finite());
        }
    }

    #[test]
    fn audio_chain_set_cw_frequency() {
        let mut chain = AudioChain::new_cw(700.0, CwBandwidth::Hz400);
        chain.set_cw_frequency(600.0);
        // Should not panic
    }

    #[test]
    fn audio_chain_set_cw_bandwidth() {
        let mut chain = AudioChain::new_cw(700.0, CwBandwidth::Hz400);
        chain.set_cw_bandwidth(CwBandwidth::Hz200);
        // Should not panic
    }

    #[test]
    fn audio_chain_set_ssb_bandwidth() {
        let mut chain = AudioChain::new_ssb(SsbBandwidth::Standard);
        chain.set_ssb_bandwidth(SsbBandwidth::Narrow);
        // Should not panic
    }

    #[test]
    fn audio_chain_set_am_bandwidth() {
        let mut chain = AudioChain::new_am(AmBandwidth::Standard);
        chain.set_am_bandwidth(AmBandwidth::Wide);
        // Should not panic
    }

    #[test]
    fn audio_chain_reset() {
        let mut chain = AudioChain::new_ssb(SsbBandwidth::Standard);

        // Process some signal
        for _ in 0..100 {
            chain.process(0.5);
        }

        chain.reset();
        // Should not panic
    }

    #[test]
    fn notch_filter_creation() {
        let notch = NotchFilter::new(1000.0);
        assert_eq!(notch.frequency(), 1000.0);
        assert!(notch.is_enabled());
    }

    #[test]
    fn notch_filter_default() {
        let notch = NotchFilter::default();
        assert_eq!(notch.frequency(), 1000.0);
    }

    #[test]
    fn notch_filter_process() {
        let mut notch = NotchFilter::new(1000.0);
        let output = notch.process(0.5);
        assert!(output.is_finite());
    }

    #[test]
    fn notch_filter_enable_disable() {
        let mut notch = NotchFilter::new(1000.0);

        notch.set_enabled(false);
        assert!(!notch.is_enabled());

        // Disabled filter passes through
        let output = notch.process(0.5);
        assert_eq!(output, 0.5);

        notch.set_enabled(true);
        assert!(notch.is_enabled());
    }

    #[test]
    fn notch_filter_set_frequency() {
        let mut notch = NotchFilter::new(1000.0);
        notch.set_frequency(2000.0);
        assert_eq!(notch.frequency(), 2000.0);
    }
}
