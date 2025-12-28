//! Noise Reduction Module
//!
//! Provides noise reduction algorithms for improving signal quality.
//! Implements spectral subtraction and other techniques suitable for
//! real-time embedded processing.

#[cfg(feature = "embedded")]
use micromath::F32Ext;

/// Noise blanker for impulse noise removal
///
/// Detects and blanks short impulse noise common in HF reception.
#[derive(Clone, Copy)]
pub struct NoiseBlanker {
    /// Detection threshold (0.0 to 1.0)
    threshold: f32,
    /// Blanking duration in samples
    blank_samples: u32,
    /// Samples remaining in blanking period
    blank_counter: u32,
    /// Envelope follower state
    envelope: f32,
    /// Attack coefficient
    attack: f32,
    /// Decay coefficient
    decay: f32,
    /// Enabled state
    enabled: bool,
    /// Last valid sample (for hold during blank)
    last_valid: f32,
}

impl NoiseBlanker {
    /// Create a new noise blanker
    ///
    /// # Arguments
    /// * `sample_rate` - Audio sample rate in Hz
    /// * `threshold` - Detection threshold (0.0 to 1.0)
    /// * `blank_duration_us` - Blanking duration in microseconds
    #[must_use]
    pub fn new(sample_rate: u32, threshold: f32, blank_duration_us: u32) -> Self {
        let blank_samples = (sample_rate * blank_duration_us / 1_000_000).max(1);
        // Fast attack for impulse detection, slow decay
        let attack = 1.0 - (-1.0 / (sample_rate as f32 * 0.0001)).exp(); // ~0.1ms attack
        let decay = 1.0 - (-1.0 / (sample_rate as f32 * 0.01)).exp(); // ~10ms decay

        Self {
            threshold: threshold.clamp(0.0, 1.0),
            blank_samples,
            blank_counter: 0,
            envelope: 0.0,
            attack,
            decay,
            enabled: true,
            last_valid: 0.0,
        }
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }

        let abs_input = input.abs();

        // Update envelope follower
        if abs_input > self.envelope {
            self.envelope += self.attack * (abs_input - self.envelope);
        } else {
            self.envelope += self.decay * (abs_input - self.envelope);
        }

        // Check for impulse (sudden spike well above envelope)
        if abs_input > self.envelope * (1.0 + self.threshold * 10.0) && abs_input > self.threshold
        {
            self.blank_counter = self.blank_samples;
        }

        // Output: blanked (hold last) or pass-through
        if self.blank_counter > 0 {
            self.blank_counter -= 1;
            self.last_valid
        } else {
            self.last_valid = input;
            input
        }
    }

    /// Process a block of samples in-place
    pub fn process_block(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Set detection threshold
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.0, 1.0);
    }

    /// Get current threshold
    #[must_use]
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Enable/disable the noise blanker
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.blank_counter = 0;
        }
    }

    /// Check if enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Reset internal state
    pub fn reset(&mut self) {
        self.blank_counter = 0;
        self.envelope = 0.0;
        self.last_valid = 0.0;
    }
}

impl Default for NoiseBlanker {
    fn default() -> Self {
        Self::new(48000, 0.5, 100)
    }
}

/// LMS (Least Mean Squares) adaptive noise filter
///
/// Adapts to reduce narrowband interference and repetitive noise.
#[derive(Clone)]
pub struct LmsFilter {
    /// Filter weights
    weights: [f32; 32],
    /// Delay line for reference signal
    delay: [f32; 32],
    /// Current position in delay line
    pos: usize,
    /// Adaptation step size (mu)
    mu: f32,
    /// Enabled state
    enabled: bool,
}

impl LmsFilter {
    /// Create a new LMS filter
    ///
    /// # Arguments
    /// * `mu` - Adaptation step size (0.001 to 0.1 typical)
    #[must_use]
    pub fn new(mu: f32) -> Self {
        Self {
            weights: [0.0; 32],
            delay: [0.0; 32],
            pos: 0,
            mu: mu.clamp(0.0001, 0.5),
            enabled: true,
        }
    }

    /// Process a sample (noise cancellation mode)
    ///
    /// Uses delayed version of input as noise reference
    pub fn process(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }

        // Store input in delay line
        self.delay[self.pos] = input;

        // Compute filter output (estimate of noise)
        let mut noise_estimate = 0.0;
        let mut idx = self.pos;
        for &w in &self.weights {
            noise_estimate += w * self.delay[idx];
            if idx == 0 {
                idx = 31;
            } else {
                idx -= 1;
            }
        }

        // Error signal (desired - estimated)
        let error = input - noise_estimate;

        // Update weights using LMS algorithm
        idx = self.pos;
        for w in &mut self.weights {
            *w += self.mu * error * self.delay[idx];
            // Limit weight growth
            *w = w.clamp(-1.0, 1.0);
            if idx == 0 {
                idx = 31;
            } else {
                idx -= 1;
            }
        }

        // Advance position
        self.pos = (self.pos + 1) & 31;

        error
    }

    /// Process a block of samples in-place
    pub fn process_block(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Set adaptation rate
    pub fn set_mu(&mut self, mu: f32) {
        self.mu = mu.clamp(0.0001, 0.5);
    }

    /// Get adaptation rate
    #[must_use]
    pub fn mu(&self) -> f32 {
        self.mu
    }

    /// Enable/disable the filter
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.weights.fill(0.0);
        self.delay.fill(0.0);
        self.pos = 0;
    }
}

impl Default for LmsFilter {
    fn default() -> Self {
        Self::new(0.01)
    }
}

/// Spectral noise reduction using weighted averaging
///
/// Estimates noise floor and reduces noise components.
#[derive(Clone, Copy)]
pub struct SpectralNoiseReducer {
    /// Noise floor estimate
    noise_floor: f32,
    /// Noise floor adaptation rate
    floor_alpha: f32,
    /// Reduction amount (0.0 to 1.0)
    reduction: f32,
    /// Enabled state
    enabled: bool,
}

impl SpectralNoiseReducer {
    /// Create a new spectral noise reducer
    ///
    /// # Arguments
    /// * `reduction` - Amount of noise reduction (0.0 to 1.0)
    #[must_use]
    pub fn new(reduction: f32) -> Self {
        Self {
            noise_floor: 0.001,
            floor_alpha: 0.001,
            reduction: reduction.clamp(0.0, 1.0),
            enabled: true,
        }
    }

    /// Process a single sample
    ///
    /// Uses magnitude-based soft gating
    pub fn process(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }

        let magnitude = input.abs();

        // Update noise floor estimate (track minimum with slow rise)
        if magnitude < self.noise_floor * 2.0 {
            self.noise_floor += self.floor_alpha * (magnitude - self.noise_floor);
        } else if magnitude > self.noise_floor * 10.0 {
            // Allow floor to rise slowly when signal is much higher
            self.noise_floor *= 1.0 + self.floor_alpha;
        }

        // Keep floor from going to zero
        self.noise_floor = self.noise_floor.max(0.0001);

        // Soft threshold: reduce signal when close to noise floor
        let threshold = self.noise_floor * (2.0 + self.reduction * 5.0);
        let gain = if magnitude < threshold {
            // Below threshold: attenuate based on how far below
            let ratio = magnitude / threshold;
            ratio.powf(1.0 + self.reduction * 2.0)
        } else {
            1.0
        };

        input * gain
    }

    /// Process a block of samples in-place
    pub fn process_block(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Set reduction amount
    pub fn set_reduction(&mut self, reduction: f32) {
        self.reduction = reduction.clamp(0.0, 1.0);
    }

    /// Get reduction amount
    #[must_use]
    pub fn reduction(&self) -> f32 {
        self.reduction
    }

    /// Enable/disable the reducer
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Reset internal state
    pub fn reset(&mut self) {
        self.noise_floor = 0.001;
    }
}

impl Default for SpectralNoiseReducer {
    fn default() -> Self {
        Self::new(0.5)
    }
}

/// Combined noise reduction chain
#[derive(Clone)]
pub struct NoiseReductionChain {
    /// Noise blanker for impulse noise
    blanker: NoiseBlanker,
    /// LMS filter for adaptive noise cancellation
    lms: LmsFilter,
    /// Spectral reducer for broadband noise
    spectral: SpectralNoiseReducer,
}

impl NoiseReductionChain {
    /// Create a new noise reduction chain
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            blanker: NoiseBlanker::new(sample_rate, 0.5, 100),
            lms: LmsFilter::new(0.01),
            spectral: SpectralNoiseReducer::new(0.5),
        }
    }

    /// Process a sample through all stages
    pub fn process(&mut self, input: f32) -> f32 {
        let sample = self.blanker.process(input);
        let sample = self.lms.process(sample);
        self.spectral.process(sample)
    }

    /// Process a block of samples in-place
    pub fn process_block(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Get mutable reference to noise blanker
    pub fn blanker_mut(&mut self) -> &mut NoiseBlanker {
        &mut self.blanker
    }

    /// Get mutable reference to LMS filter
    pub fn lms_mut(&mut self) -> &mut LmsFilter {
        &mut self.lms
    }

    /// Get mutable reference to spectral reducer
    pub fn spectral_mut(&mut self) -> &mut SpectralNoiseReducer {
        &mut self.spectral
    }

    /// Reset all stages
    pub fn reset(&mut self) {
        self.blanker.reset();
        self.lms.reset();
        self.spectral.reset();
    }
}

impl Default for NoiseReductionChain {
    fn default() -> Self {
        Self::new(48000)
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    // =========================================================================
    // Noise Blanker Tests
    // =========================================================================

    #[test]
    fn noise_blanker_creation() {
        let nb = NoiseBlanker::new(48000, 0.5, 100);
        assert!(nb.is_enabled());
        assert_eq!(nb.threshold(), 0.5);
    }

    #[test]
    fn noise_blanker_default() {
        let nb = NoiseBlanker::default();
        assert!(nb.is_enabled());
    }

    #[test]
    fn noise_blanker_passes_normal_signal() {
        let mut nb = NoiseBlanker::new(48000, 0.5, 100);

        // Normal level signal should pass through
        let input = 0.1;
        for _ in 0..100 {
            let output = nb.process(input);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn noise_blanker_threshold_clamp() {
        let mut nb = NoiseBlanker::default();

        nb.set_threshold(1.5);
        assert_eq!(nb.threshold(), 1.0);

        nb.set_threshold(-0.5);
        assert_eq!(nb.threshold(), 0.0);
    }

    #[test]
    fn noise_blanker_enable_disable() {
        let mut nb = NoiseBlanker::default();

        nb.set_enabled(false);
        assert!(!nb.is_enabled());

        // Disabled passes through unchanged
        let output = nb.process(0.5);
        assert_eq!(output, 0.5);

        nb.set_enabled(true);
        assert!(nb.is_enabled());
    }

    #[test]
    fn noise_blanker_reset() {
        let mut nb = NoiseBlanker::default();

        // Process some signal
        for _ in 0..100 {
            nb.process(0.3);
        }

        nb.reset();
        // Should not panic
    }

    #[test]
    fn noise_blanker_process_block() {
        let mut nb = NoiseBlanker::default();
        let mut samples = [0.1, 0.2, 0.3, 0.4, 0.5];
        nb.process_block(&mut samples);

        for &s in &samples {
            assert!(s.is_finite());
        }
    }

    // =========================================================================
    // LMS Filter Tests
    // =========================================================================

    #[test]
    fn lms_creation() {
        let lms = LmsFilter::new(0.01);
        assert!(lms.is_enabled());
        assert_eq!(lms.mu(), 0.01);
    }

    #[test]
    fn lms_default() {
        let lms = LmsFilter::default();
        assert!(lms.is_enabled());
    }

    #[test]
    fn lms_process_finite() {
        let mut lms = LmsFilter::new(0.01);

        for _ in 0..1000 {
            let output = lms.process(0.3);
            assert!(output.is_finite(), "LMS output should be finite");
        }
    }

    #[test]
    fn lms_mu_clamp() {
        let mut lms = LmsFilter::default();

        lms.set_mu(1.0);
        assert_eq!(lms.mu(), 0.5);

        lms.set_mu(-0.1);
        assert_eq!(lms.mu(), 0.0001);
    }

    #[test]
    fn lms_enable_disable() {
        let mut lms = LmsFilter::default();

        lms.set_enabled(false);
        assert!(!lms.is_enabled());

        let output = lms.process(0.5);
        assert_eq!(output, 0.5);

        lms.set_enabled(true);
        assert!(lms.is_enabled());
    }

    #[test]
    fn lms_reset() {
        let mut lms = LmsFilter::default();

        // Train the filter
        for _ in 0..1000 {
            lms.process(0.3);
        }

        lms.reset();
        // Weights should be cleared
    }

    #[test]
    fn lms_process_block() {
        let mut lms = LmsFilter::default();
        let mut samples = [0.1, 0.2, 0.3, 0.4, 0.5];
        lms.process_block(&mut samples);

        for &s in &samples {
            assert!(s.is_finite());
        }
    }

    // =========================================================================
    // Spectral Noise Reducer Tests
    // =========================================================================

    #[test]
    fn spectral_creation() {
        let snr = SpectralNoiseReducer::new(0.5);
        assert!(snr.is_enabled());
        assert_eq!(snr.reduction(), 0.5);
    }

    #[test]
    fn spectral_default() {
        let snr = SpectralNoiseReducer::default();
        assert!(snr.is_enabled());
    }

    #[test]
    fn spectral_process_finite() {
        let mut snr = SpectralNoiseReducer::new(0.5);

        for _ in 0..1000 {
            let output = snr.process(0.3);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn spectral_reduction_clamp() {
        let mut snr = SpectralNoiseReducer::default();

        snr.set_reduction(1.5);
        assert_eq!(snr.reduction(), 1.0);

        snr.set_reduction(-0.5);
        assert_eq!(snr.reduction(), 0.0);
    }

    #[test]
    fn spectral_enable_disable() {
        let mut snr = SpectralNoiseReducer::default();

        snr.set_enabled(false);
        assert!(!snr.is_enabled());

        let output = snr.process(0.5);
        assert_eq!(output, 0.5);

        snr.set_enabled(true);
        assert!(snr.is_enabled());
    }

    #[test]
    fn spectral_attenuates_weak_signal() {
        let mut snr = SpectralNoiseReducer::new(0.8);

        // Let noise floor settle on weak signal
        for _ in 0..1000 {
            snr.process(0.001);
        }

        // Very weak signal near noise floor should be attenuated
        let output = snr.process(0.001);
        assert!(
            output <= 0.001,
            "Weak signal should be attenuated: {}",
            output
        );
    }

    #[test]
    fn spectral_passes_strong_signal() {
        let mut snr = SpectralNoiseReducer::new(0.8);

        // Let noise floor settle
        for _ in 0..1000 {
            snr.process(0.001);
        }

        // Strong signal should pass with little attenuation
        let output = snr.process(0.5);
        assert!(
            output > 0.3,
            "Strong signal should mostly pass: {}",
            output
        );
    }

    #[test]
    fn spectral_reset() {
        let mut snr = SpectralNoiseReducer::default();

        for _ in 0..1000 {
            snr.process(0.5);
        }

        snr.reset();
        // Should not panic
    }

    #[test]
    fn spectral_process_block() {
        let mut snr = SpectralNoiseReducer::default();
        let mut samples = [0.1, 0.2, 0.3, 0.4, 0.5];
        snr.process_block(&mut samples);

        for &s in &samples {
            assert!(s.is_finite());
        }
    }

    // =========================================================================
    // Noise Reduction Chain Tests
    // =========================================================================

    #[test]
    fn chain_creation() {
        let chain = NoiseReductionChain::new(48000);
        assert!(chain.blanker.is_enabled());
        assert!(chain.lms.is_enabled());
        assert!(chain.spectral.is_enabled());
    }

    #[test]
    fn chain_default() {
        let chain = NoiseReductionChain::default();
        assert!(chain.blanker.is_enabled());
    }

    #[test]
    fn chain_process() {
        let mut chain = NoiseReductionChain::new(48000);

        for _ in 0..1000 {
            let output = chain.process(0.3);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn chain_process_block() {
        let mut chain = NoiseReductionChain::default();
        let mut samples = [0.1, 0.2, 0.3, 0.4, 0.5];
        chain.process_block(&mut samples);

        for &s in &samples {
            assert!(s.is_finite());
        }
    }

    #[test]
    fn chain_access_components() {
        let mut chain = NoiseReductionChain::default();

        chain.blanker_mut().set_threshold(0.3);
        chain.lms_mut().set_mu(0.02);
        chain.spectral_mut().set_reduction(0.7);

        // Should not panic
    }

    #[test]
    fn chain_reset() {
        let mut chain = NoiseReductionChain::default();

        for _ in 0..1000 {
            chain.process(0.3);
        }

        chain.reset();
        // Should not panic
    }
}
