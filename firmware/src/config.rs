//! System configuration and hardware constants
//!
//! This module defines compile-time constants for the SDR transceiver hardware.
//! All pin mappings, clock frequencies, and hardware parameters are centralized here.

use crate::types::{Frequency, Mode, TuningStep};

/// System clock frequency (STM32G474 @ 170MHz)
pub const SYSTEM_CLOCK_HZ: u32 = 170_000_000;

/// ADC sample rate for audio (48 kHz)
pub const AUDIO_SAMPLE_RATE: u32 = 48_000;

/// ADC sample rate for IQ data (192 kHz for wideband)
pub const IQ_SAMPLE_RATE: u32 = 192_000;

/// DAC sample rate (matches audio rate)
pub const DAC_SAMPLE_RATE: u32 = 48_000;

/// I2C bus frequency for `Si5351A` and display
pub const I2C_FREQUENCY_HZ: u32 = 400_000;

/// `Si5351A` I2C address
pub const SI5351_I2C_ADDR: u8 = 0x60;

/// SSD1306 OLED I2C address
pub const DISPLAY_I2C_ADDR: u8 = 0x3C;

/// Display width in pixels
pub const DISPLAY_WIDTH: u32 = 128;

/// Display height in pixels
pub const DISPLAY_HEIGHT: u32 = 64;

/// `Si5351A` crystal frequency (25 MHz standard)
pub const SI5351_XTAL_FREQ: u32 = 25_000_000;

/// Number of LPF filter banks
pub const NUM_LPF_BANKS: usize = 5;

/// T/R relay switching delay in microseconds
pub const TR_RELAY_DELAY_US: u32 = 10_000;

/// SWR protection threshold (3:1)
pub const SWR_PROTECTION_THRESHOLD: f32 = 3.0;

/// Maximum transmit power in watts
pub const MAX_TX_POWER_WATTS: f32 = 5.0;

/// USB VID (use test VID for development)
pub const USB_VID: u16 = 0x1209;

/// USB PID (get from pid.codes for production)
pub const USB_PID: u16 = 0x0001;

/// Default startup frequency (40m band, FT8 frequency)
pub const DEFAULT_FREQUENCY_HZ: u32 = 7_074_000;

/// Default operating mode
pub const DEFAULT_MODE: Mode = Mode::Usb;

/// Default tuning step
pub const DEFAULT_TUNING_STEP: TuningStep = TuningStep::KHz1;

/// Audio buffer size in samples
pub const AUDIO_BUFFER_SIZE: usize = 256;

/// IQ buffer size in samples (I + Q interleaved)
pub const IQ_BUFFER_SIZE: usize = 512;

/// Number of FIR filter taps for audio processing
pub const FIR_TAPS: usize = 127;

/// AGC attack time constant in milliseconds
pub const AGC_ATTACK_MS: u32 = 10;

/// AGC decay time constant in milliseconds
pub const AGC_DECAY_MS: u32 = 500;

/// Encoder debounce time in milliseconds
pub const ENCODER_DEBOUNCE_MS: u32 = 2;

/// Button debounce time in milliseconds
pub const BUTTON_DEBOUNCE_MS: u32 = 50;

/// CAT command buffer size
pub const CAT_BUFFER_SIZE: usize = 64;

/// USB CDC ACM packet size
pub const USB_CDC_PACKET_SIZE: u16 = 64;

/// Pin assignments for GPIO
pub mod pins {
    //! GPIO pin assignments matching the schematic

    /// Status LED (directly on MCU)
    pub const LED_STATUS: &str = "PA5";

    /// I2C1 SCL (Si5351, Display)
    pub const I2C1_SCL: &str = "PB6";

    /// I2C1 SDA (Si5351, Display)
    pub const I2C1_SDA: &str = "PB7";

    /// Encoder A input
    pub const ENCODER_A: &str = "PA0";

    /// Encoder B input
    pub const ENCODER_B: &str = "PA1";

    /// Encoder push button
    pub const ENCODER_SW: &str = "PA2";

    /// PTT input (active low)
    pub const PTT_IN: &str = "PA3";

    /// T/R relay control
    pub const TR_RELAY: &str = "PB0";

    /// LPF bank select bit 0
    pub const LPF_SEL0: &str = "PC0";

    /// LPF bank select bit 1
    pub const LPF_SEL1: &str = "PC1";

    /// LPF bank select bit 2
    pub const LPF_SEL2: &str = "PC2";

    /// Audio ADC input
    pub const AUDIO_ADC: &str = "PA4";

    /// Audio DAC output
    pub const AUDIO_DAC: &str = "PA5";

    /// Forward power ADC
    pub const FWD_POWER: &str = "PB1";

    /// Reflected power ADC
    pub const REF_POWER: &str = "PB2";

    /// USB D+ (handled by USB peripheral)
    pub const USB_DP: &str = "PA12";

    /// USB D- (handled by USB peripheral)
    pub const USB_DM: &str = "PA11";

    /// USB-C CC1 for UCPD
    pub const USB_CC1: &str = "PB4";

    /// USB-C CC2 for UCPD
    pub const USB_CC2: &str = "PB5";

    /// PA drive PWM output
    pub const PA_DRIVE: &str = "PA8";

    /// Class-E H-bridge A high side
    pub const PA_AH: &str = "PA9";

    /// Class-E H-bridge A low side
    pub const PA_AL: &str = "PA10";

    /// Class-E H-bridge B high side
    pub const PA_BH: &str = "PB13";

    /// Class-E H-bridge B low side
    pub const PA_BL: &str = "PB14";
}

/// DMA channel assignments
pub mod dma {
    //! DMA channel assignments for zero-copy transfers

    /// I2C1 TX DMA channel
    pub const I2C1_TX: u8 = 1;

    /// I2C1 RX DMA channel
    pub const I2C1_RX: u8 = 2;

    /// ADC1 DMA channel (audio input)
    pub const ADC1: u8 = 3;

    /// DAC1 DMA channel (audio output)
    pub const DAC1: u8 = 4;

    /// ADC2 DMA channel (IQ input)
    pub const ADC2: u8 = 5;
}

/// Timer assignments
pub mod timers {
    //! Hardware timer assignments

    /// Audio sample rate timer (ADC/DAC trigger)
    pub const AUDIO_SAMPLE: u8 = 2;

    /// IQ sample rate timer (ADC2 trigger)
    pub const IQ_SAMPLE: u8 = 3;

    /// Encoder timer (quadrature mode)
    pub const ENCODER: u8 = 4;

    /// PA PWM timer (Class-E drive)
    pub const PA_PWM: u8 = 1;

    /// General purpose timer for delays
    pub const GENERAL: u8 = 6;
}

/// Build the default startup frequency
#[must_use]
pub const fn default_frequency() -> Option<Frequency> {
    Frequency::from_hz(DEFAULT_FREQUENCY_HZ)
}
