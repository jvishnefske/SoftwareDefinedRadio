//! DAC Driver
//!
//! Provides audio output through the STM32G474 DAC peripheral.
//! Uses DMA for continuous audio playback without CPU intervention.

use embassy_stm32::dac::{DacChannel, Value};

use crate::config::AUDIO_BUFFER_SIZE;

/// Audio output sample
#[derive(Clone, Copy, Debug)]
pub struct DacSample {
    /// 12-bit DAC value (0-4095)
    value: u16,
}

impl DacSample {
    /// Create from raw 12-bit value
    #[must_use]
    pub const fn from_raw(value: u16) -> Self {
        Self {
            value: if value > 4095 { 4095 } else { value },
        }
    }

    /// Create from signed audio sample (-1.0 to 1.0)
    #[must_use]
    pub fn from_audio(sample: f32) -> Self {
        let clamped = sample.clamp(-1.0, 1.0);
        let raw = ((clamped + 1.0) * 2047.5) as u16;
        Self::from_raw(raw)
    }

    /// Create from signed 16-bit audio
    #[must_use]
    pub const fn from_i16(sample: i16) -> Self {
        let shifted = (sample as i32 + 32768) / 16;
        Self::from_raw(shifted as u16)
    }

    /// Get the raw 12-bit value
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.value
    }

    /// Convert to embassy DAC value
    #[must_use]
    pub const fn as_dac_value(self) -> Value {
        Value::Bit12Left(self.value)
    }
}

impl Default for DacSample {
    fn default() -> Self {
        Self::from_raw(2048) // Mid-scale (0V with bias)
    }
}

impl defmt::Format for DacSample {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "DAC({})", self.value);
    }
}

/// Audio DAC output driver
pub struct AudioDac<'d, T: embassy_stm32::dac::Instance> {
    channel: DacChannel<'d, T, 1>,
}

impl<'d, T: embassy_stm32::dac::Instance> AudioDac<'d, T> {
    /// Create a new audio DAC driver
    #[must_use] 
    pub fn new(channel: DacChannel<'d, T, 1>) -> Self {
        Self { channel }
    }

    /// Write a single sample to the DAC
    pub fn write(&mut self, sample: DacSample) {
        self.channel.set(sample.as_dac_value());
    }

    /// Trigger DAC conversion
    pub fn trigger(&mut self) {
        self.channel.trigger();
    }
}

/// Output audio buffer for DMA transfers
pub struct OutputBuffer {
    /// Sample buffer (12-bit values)
    samples: [u16; AUDIO_BUFFER_SIZE],
    /// Write position
    write_pos: usize,
}

impl OutputBuffer {
    /// Create a new empty output buffer
    #[must_use]
    pub const fn new() -> Self {
        Self {
            samples: [2048; AUDIO_BUFFER_SIZE], // Mid-scale
            write_pos: 0,
        }
    }

    /// Add a sample to the buffer
    pub fn push(&mut self, sample: DacSample) -> bool {
        if self.write_pos >= AUDIO_BUFFER_SIZE {
            return false;
        }
        self.samples[self.write_pos] = sample.raw();
        self.write_pos += 1;
        true
    }

    /// Reset the buffer for new data
    pub fn reset(&mut self) {
        self.write_pos = 0;
    }

    /// Check if buffer is full
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.write_pos >= AUDIO_BUFFER_SIZE
    }

    /// Get the buffer as a slice for DMA
    #[must_use]
    pub fn as_slice(&self) -> &[u16] {
        &self.samples[..self.write_pos]
    }

    /// Fill buffer from i16 audio samples
    pub fn fill_from_i16(&mut self, samples: &[i16]) {
        self.reset();
        for &sample in samples.iter().take(AUDIO_BUFFER_SIZE) {
            self.push(DacSample::from_i16(sample));
        }
    }
}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Double buffer for continuous DMA playback
pub struct DoubleBuffer {
    /// Front buffer (currently being played)
    front: OutputBuffer,
    /// Back buffer (being filled)
    back: OutputBuffer,
    /// Which buffer is front
    front_is_a: bool,
}

impl DoubleBuffer {
    /// Create a new double buffer
    #[must_use]
    pub const fn new() -> Self {
        Self {
            front: OutputBuffer::new(),
            back: OutputBuffer::new(),
            front_is_a: true,
        }
    }

    /// Get mutable reference to back buffer for filling
    #[must_use]
    pub fn back_mut(&mut self) -> &mut OutputBuffer {
        if self.front_is_a {
            &mut self.back
        } else {
            &mut self.front
        }
    }

    /// Get reference to front buffer for DMA
    #[must_use]
    pub fn front(&self) -> &OutputBuffer {
        if self.front_is_a {
            &self.front
        } else {
            &self.back
        }
    }

    /// Swap front and back buffers
    pub fn swap(&mut self) {
        self.front_is_a = !self.front_is_a;
    }
}

impl Default for DoubleBuffer {
    fn default() -> Self {
        Self::new()
    }
}
