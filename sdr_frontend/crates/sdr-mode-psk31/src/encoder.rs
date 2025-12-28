//! PSK31 encoder implementation.

use crate::varicode::VaricodeEncoder;
use core::f32::consts::PI;
#[allow(unused_imports)]
use micromath::F32Ext;

/// PSK31 encoder configuration.
#[derive(Clone, Debug)]
pub struct Psk31EncoderConfig {
    /// Sample rate in Hz
    pub sample_rate: f32,
    /// Carrier frequency in Hz
    pub carrier_freq_hz: f32,
    /// Output amplitude (0.0 to 1.0)
    pub amplitude: f32,
    /// QPSK mode (false = BPSK)
    pub qpsk_mode: bool,
}

impl Default for Psk31EncoderConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000.0,
            carrier_freq_hz: 1500.0,
            amplitude: 0.5,
            qpsk_mode: false,
        }
    }
}

/// PSK31 encoder state.
pub struct Psk31Encoder {
    config: Psk31EncoderConfig,

    // Varicode encoder
    varicode: VaricodeEncoder,

    // Carrier phase
    phase: f32,
    phase_inc: f32,

    // Current symbol phase (0 or π for BPSK)
    symbol_phase: f32,
    target_phase: f32,

    // Symbol timing
    samples_per_symbol: f32,
    sample_count: f32,

    // Raised cosine shaping
    shaping_phase: f32,

    // Idle preamble
    idle_count: u32,
}

impl Psk31Encoder {
    /// Create a new PSK31 encoder.
    #[must_use]
    pub fn new(config: Psk31EncoderConfig) -> Self {
        const BAUD_RATE: f32 = 31.25;
        let samples_per_symbol = config.sample_rate / BAUD_RATE;
        let phase_inc = 2.0 * PI * config.carrier_freq_hz / config.sample_rate;

        Self {
            config,
            varicode: VaricodeEncoder::new(),
            phase: 0.0,
            phase_inc,
            symbol_phase: 0.0,
            target_phase: 0.0,
            samples_per_symbol,
            sample_count: 0.0,
            shaping_phase: 0.0,
            idle_count: 0,
        }
    }

    /// Queue text for transmission.
    pub fn queue_text(&mut self, text: &str) {
        self.varicode.queue_string(text);
    }

    /// Queue a single character.
    pub fn queue_char(&mut self, ch: char) {
        self.varicode.queue_char(ch);
    }

    /// Generate next audio sample.
    ///
    /// Returns `None` when idle (nothing to transmit).
    pub fn next_sample(&mut self) -> Option<f32> {
        // Check if we need a new bit
        self.sample_count += 1.0;
        if self.sample_count >= self.samples_per_symbol {
            self.sample_count -= self.samples_per_symbol;

            // Get next bit
            match self.varicode.next_bit() {
                Some(bit) => {
                    // BPSK: 0 = 180° phase shift, 1 = no change
                    if !bit {
                        self.target_phase += PI;
                        if self.target_phase > PI {
                            self.target_phase -= 2.0 * PI;
                        }
                    }
                    self.shaping_phase = 0.0;
                    self.idle_count = 0;
                }
                None => {
                    // Idle - send preamble (continuous carrier)
                    self.idle_count += 1;
                    if self.idle_count > 10 {
                        // Stop transmitting after idle period
                        return None;
                    }
                }
            }
        }

        // Raised cosine phase transition
        let transition_progress = self.shaping_phase / self.samples_per_symbol;
        let shaping = if transition_progress < 1.0 {
            // Raised cosine transition
            0.5 * (1.0 - (PI * transition_progress).cos())
        } else {
            1.0
        };
        self.shaping_phase += 1.0;

        // Interpolate phase
        let current_phase = self.symbol_phase + shaping * (self.target_phase - self.symbol_phase);

        // Generate carrier with phase modulation
        let output = self.config.amplitude * (self.phase + current_phase).cos();

        // Advance carrier phase
        self.phase += self.phase_inc;
        if self.phase > PI {
            self.phase -= 2.0 * PI;
        }

        // Update symbol phase at end of transition
        if transition_progress >= 1.0 {
            self.symbol_phase = self.target_phase;
        }

        Some(output)
    }

    /// Check if encoder is idle.
    ///
    /// Returns true if nothing is queued and either:
    /// - Never started transmitting (fresh encoder)
    /// - Finished transmitting and went through idle period
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.varicode.is_idle() && (self.idle_count > 10 || self.sample_count == 0.0)
    }

    /// Clear the transmit queue.
    pub fn clear(&mut self) {
        self.varicode.clear();
        self.idle_count = 100; // Force idle
    }

    /// Reset encoder state.
    pub fn reset(&mut self) {
        self.varicode.clear();
        self.phase = 0.0;
        self.symbol_phase = 0.0;
        self.target_phase = 0.0;
        self.sample_count = 0.0;
        self.shaping_phase = 0.0;
        self.idle_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_generates_samples() {
        let mut encoder = Psk31Encoder::new(Psk31EncoderConfig::default());
        encoder.queue_char('e');

        let mut sample_count = 0usize;
        let mut last_sample = 0.0f32;
        while let Some(sample) = encoder.next_sample() {
            last_sample = sample;
            sample_count += 1;
            if sample_count > 10000 {
                break; // Safety limit
            }
        }

        // Should generate samples for 'e' + idle
        assert!(sample_count > 0);
        // Verify samples are in valid range
        assert!(last_sample.abs() <= 1.0);
    }

    #[test]
    fn test_encoder_idle_when_empty() {
        let encoder = Psk31Encoder::new(Psk31EncoderConfig::default());
        assert!(encoder.is_idle());
    }
}
