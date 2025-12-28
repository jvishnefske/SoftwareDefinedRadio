//! ADC Driver
//!
//! Provides async ADC reading for audio input and power measurement.
//! Uses DMA for efficient bulk transfers of audio samples.

use embassy_stm32::adc::{Adc, AdcChannel, SampleTime};
use embassy_stm32::peripherals::{ADC1, ADC2};
use micromath::F32Ext;

use crate::config::{AUDIO_BUFFER_SIZE, IQ_BUFFER_SIZE};

/// ADC reading result
#[derive(Clone, Copy, Debug)]
pub struct AdcReading {
    /// Raw 12-bit ADC value (0-4095)
    raw: u16,
}

impl AdcReading {
    /// Create a new ADC reading from raw value
    #[must_use]
    pub const fn from_raw(raw: u16) -> Self {
        Self { raw }
    }

    /// Get the raw 12-bit value
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.raw
    }

    /// Convert to voltage (assuming 3.3V reference)
    #[must_use]
    pub fn as_voltage(self) -> f32 {
        (f32::from(self.raw) / 4095.0) * 3.3
    }

    /// Convert to signed audio sample (-1.0 to 1.0)
    #[must_use]
    pub fn as_audio_sample(self) -> f32 {
        (f32::from(self.raw) / 2047.5) - 1.0
    }

    /// Convert to signed 16-bit audio sample
    #[must_use]
    pub fn as_i16(self) -> i16 {
        (i32::from(self.raw) - 2048).clamp(-32768, 32767) as i16 * 16
    }
}

impl defmt::Format for AdcReading {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "ADC({})", self.raw);
    }
}

/// Audio ADC driver for receiving audio samples
pub struct AudioAdc<'d> {
    adc: Adc<'d, ADC1>,
}

impl AudioAdc<'_> {
    /// Create a new audio ADC driver
    #[must_use] 
    pub fn new(adc: ADC1) -> Self {
        let adc = Adc::new(adc);
        Self { adc }
    }

    /// Configure the ADC for audio sampling
    pub fn configure(&mut self) {
        self.adc.set_sample_time(SampleTime::CYCLES247_5);
    }

    /// Read a single audio sample
    pub fn read<T: AdcChannel<ADC1>>(&mut self, channel: &mut T) -> AdcReading {
        let raw = self.adc.blocking_read(channel);
        AdcReading::from_raw(raw)
    }
}

/// IQ ADC driver for quadrature sampling detector
pub struct IqAdc<'d> {
    adc: Adc<'d, ADC2>,
}

impl IqAdc<'_> {
    /// Create a new IQ ADC driver
    #[must_use] 
    pub fn new(adc: ADC2) -> Self {
        let adc = Adc::new(adc);
        Self { adc }
    }

    /// Configure the ADC for IQ sampling
    pub fn configure(&mut self) {
        self.adc.set_sample_time(SampleTime::CYCLES47_5);
    }

    /// Read a single IQ sample
    pub fn read<T: AdcChannel<ADC2>>(&mut self, channel: &mut T) -> AdcReading {
        let raw = self.adc.blocking_read(channel);
        AdcReading::from_raw(raw)
    }
}

/// Power measurement ADC readings
#[derive(Clone, Copy, Debug)]
pub struct PowerReading {
    /// Forward power ADC value
    pub forward: AdcReading,
    /// Reflected power ADC value
    pub reflected: AdcReading,
}

impl PowerReading {
    /// Calculate SWR from forward and reflected power
    #[must_use]
    pub fn swr_ratio(&self) -> f32 {
        let fwd = f32::from(self.forward.raw());
        let ref_pwr = f32::from(self.reflected.raw());

        if fwd < 10.0 {
            return 999.0;
        }

        let rho = (ref_pwr / fwd).sqrt().min(0.99);
        (1.0 + rho) / (1.0 - rho)
    }

    /// Estimate forward power in watts
    #[must_use]
    pub fn forward_watts(&self, cal_factor: f32) -> f32 {
        let v = self.forward.as_voltage();
        (v * v) * cal_factor
    }
}

impl defmt::Format for PowerReading {
    fn format(&self, f: defmt::Formatter) {
        let swr = self.swr_ratio();
        let whole = swr as u32;
        let frac = ((swr - whole as f32) * 10.0) as u32;
        defmt::write!(f, "Pwr(fwd={}, ref={}, SWR={}.{}:1)",
            self.forward.raw(), self.reflected.raw(), whole, frac);
    }
}

/// Audio sample buffer for DMA transfers
pub struct AudioBuffer {
    /// Sample buffer
    samples: [i16; AUDIO_BUFFER_SIZE],
    /// Number of valid samples
    len: usize,
}

impl AudioBuffer {
    /// Create a new empty audio buffer
    #[must_use]
    pub const fn new() -> Self {
        Self {
            samples: [0; AUDIO_BUFFER_SIZE],
            len: 0,
        }
    }

    /// Get the samples as a slice
    #[must_use]
    pub fn as_slice(&self) -> &[i16] {
        &self.samples[..self.len]
    }

    /// Get mutable access to the buffer
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [i16] {
        &mut self.samples
    }

    /// Set the number of valid samples
    pub fn set_len(&mut self, len: usize) {
        self.len = len.min(AUDIO_BUFFER_SIZE);
    }

    /// Check if buffer is full
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.len >= AUDIO_BUFFER_SIZE
    }
}

impl Default for AudioBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// IQ sample buffer for DMA transfers
pub struct IqBuffer {
    /// Interleaved I and Q samples
    samples: [i16; IQ_BUFFER_SIZE],
    /// Number of valid sample pairs
    len: usize,
}

impl IqBuffer {
    /// Create a new empty IQ buffer
    #[must_use]
    pub const fn new() -> Self {
        Self {
            samples: [0; IQ_BUFFER_SIZE],
            len: 0,
        }
    }

    /// Get the number of IQ pairs
    #[must_use]
    pub const fn num_pairs(&self) -> usize {
        self.len / 2
    }

    /// Get I sample at index
    #[must_use]
    pub fn i_sample(&self, idx: usize) -> i16 {
        self.samples[idx * 2]
    }

    /// Get Q sample at index
    #[must_use]
    pub fn q_sample(&self, idx: usize) -> i16 {
        self.samples[idx * 2 + 1]
    }

    /// Get mutable access to the buffer
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [i16] {
        &mut self.samples
    }

    /// Set the number of valid samples
    pub fn set_len(&mut self, len: usize) {
        self.len = len.min(IQ_BUFFER_SIZE);
    }
}

impl Default for IqBuffer {
    fn default() -> Self {
        Self::new()
    }
}
