//! Configuration and Constants Tests
//!
//! Tests to verify configuration values are valid and consistent.
//! Run with: cargo test --target x86_64-unknown-linux-gnu --no-default-features --features std --test config_tests

use sdr_firmware::config::*;
use sdr_firmware::types::{Band, Mode};

// =============================================================================
// Clock and Sample Rate Tests
// =============================================================================

#[test]
fn system_clock_valid() {
    // STM32G474 max clock is 170 MHz
    assert_eq!(SYSTEM_CLOCK_HZ, 170_000_000);
}

#[test]
fn audio_sample_rate_standard() {
    // 48 kHz is standard audio rate
    assert_eq!(AUDIO_SAMPLE_RATE, 48_000);
}

#[test]
fn iq_sample_rate_sufficient() {
    // IQ rate should be at least 4x max audio for quadrature
    assert!(IQ_SAMPLE_RATE >= AUDIO_SAMPLE_RATE * 4);
}

#[test]
fn dac_matches_audio() {
    assert_eq!(DAC_SAMPLE_RATE, AUDIO_SAMPLE_RATE);
}

// =============================================================================
// I2C Configuration Tests
// =============================================================================

#[test]
fn i2c_frequency_valid() {
    // Standard I2C speeds: 100kHz, 400kHz, 1MHz
    assert!(I2C_FREQUENCY_HZ == 100_000 || I2C_FREQUENCY_HZ == 400_000 || I2C_FREQUENCY_HZ == 1_000_000);
}

#[test]
fn si5351_address_valid() {
    // Si5351A default address is 0x60 or 0x61
    assert!(SI5351_I2C_ADDR == 0x60 || SI5351_I2C_ADDR == 0x61);
}

#[test]
fn display_address_valid() {
    // SSD1306 addresses are 0x3C or 0x3D
    assert!(DISPLAY_I2C_ADDR == 0x3C || DISPLAY_I2C_ADDR == 0x3D);
}

// =============================================================================
// Display Configuration Tests
// =============================================================================

#[test]
fn display_dimensions_standard() {
    // Common OLED sizes: 128x32 or 128x64
    assert_eq!(DISPLAY_WIDTH, 128);
    assert!(DISPLAY_HEIGHT == 32 || DISPLAY_HEIGHT == 64);
}

// =============================================================================
// Si5351 Configuration Tests
// =============================================================================

#[test]
fn si5351_crystal_frequency() {
    // Standard crystal is 25 MHz or 27 MHz
    assert!(SI5351_XTAL_FREQ == 25_000_000 || SI5351_XTAL_FREQ == 27_000_000);
}

// =============================================================================
// RF Configuration Tests
// =============================================================================

#[test]
fn lpf_banks_match_bands() {
    // Should have enough LPF banks for all HF bands
    // 80m, 40m, 30m/20m, 17m/15m, 12m/10m would be 5
    assert!(NUM_LPF_BANKS >= 5);
}

#[test]
fn tr_relay_delay_reasonable() {
    // T/R delay should be 5-20ms for typical relays
    assert!(TR_RELAY_DELAY_US >= 5_000);
    assert!(TR_RELAY_DELAY_US <= 50_000);
}

#[test]
fn swr_threshold_reasonable() {
    // SWR protection typically kicks in around 2:1 to 3:1
    assert!(SWR_PROTECTION_THRESHOLD >= 2.0);
    assert!(SWR_PROTECTION_THRESHOLD <= 5.0);
}

#[test]
fn max_power_reasonable() {
    // QRP is typically 5W or less
    assert!(MAX_TX_POWER_WATTS >= 0.5);
    assert!(MAX_TX_POWER_WATTS <= 10.0);
}

// =============================================================================
// USB Configuration Tests
// =============================================================================

#[test]
fn usb_vid_valid() {
    // 0x1209 is the test/hobbyist VID from pid.codes
    assert!(USB_VID != 0x0000);
}

#[test]
fn usb_pid_valid() {
    assert!(USB_PID != 0x0000);
}

#[test]
fn usb_packet_size_valid() {
    // CDC packet size must be 8, 16, 32, or 64 for full-speed USB
    assert!(USB_CDC_PACKET_SIZE == 8 || USB_CDC_PACKET_SIZE == 16 ||
            USB_CDC_PACKET_SIZE == 32 || USB_CDC_PACKET_SIZE == 64);
}

// =============================================================================
// Default Settings Tests
// =============================================================================

#[test]
fn default_frequency_valid() {
    let freq = default_frequency();
    assert!(freq.is_some(), "Default frequency should be valid");

    let freq = freq.unwrap();
    // Should be in a valid ham band
    let band = Band::from_frequency(freq);
    assert!(band.is_some(), "Default frequency should be in a band");
}

#[test]
fn default_frequency_in_40m() {
    let freq = default_frequency().unwrap();
    let band = Band::from_frequency(freq).unwrap();
    assert_eq!(band, Band::M40, "Default should be 40m FT8");
}

#[test]
fn default_mode_usb() {
    // FT8 and most digital modes use USB
    assert_eq!(DEFAULT_MODE, Mode::Usb);
}

// =============================================================================
// Buffer Size Tests
// =============================================================================

#[test]
fn audio_buffer_power_of_two() {
    // Power of 2 simplifies DMA and FFT
    assert!(AUDIO_BUFFER_SIZE.is_power_of_two());
}

#[test]
fn iq_buffer_power_of_two() {
    assert!(IQ_BUFFER_SIZE.is_power_of_two());
}

#[test]
fn iq_buffer_larger_than_audio() {
    // IQ needs more bandwidth
    assert!(IQ_BUFFER_SIZE >= AUDIO_BUFFER_SIZE);
}

#[test]
fn fir_taps_reasonable() {
    // FIR taps: more = sharper filter, more latency/CPU
    // 63-255 is typical for audio
    assert!(FIR_TAPS >= 31);
    assert!(FIR_TAPS <= 511);
}

#[test]
fn cat_buffer_sufficient() {
    // Typical CAT commands are < 20 bytes, but leave headroom
    assert!(CAT_BUFFER_SIZE >= 32);
}

// =============================================================================
// AGC Timing Tests
// =============================================================================

#[test]
fn agc_attack_fast() {
    // Attack should be fast (5-50ms)
    assert!(AGC_ATTACK_MS >= 1);
    assert!(AGC_ATTACK_MS <= 100);
}

#[test]
fn agc_decay_slow() {
    // Decay should be slow (200-2000ms)
    assert!(AGC_DECAY_MS >= 100);
    assert!(AGC_DECAY_MS <= 5000);
}

#[test]
fn agc_attack_faster_than_decay() {
    assert!(AGC_ATTACK_MS < AGC_DECAY_MS);
}

// =============================================================================
// Debounce Timing Tests
// =============================================================================

#[test]
fn encoder_debounce_fast() {
    // Encoder needs fast response (1-10ms)
    assert!(ENCODER_DEBOUNCE_MS >= 1);
    assert!(ENCODER_DEBOUNCE_MS <= 20);
}

#[test]
fn button_debounce_reasonable() {
    // Button debounce typically 20-100ms
    assert!(BUTTON_DEBOUNCE_MS >= 10);
    assert!(BUTTON_DEBOUNCE_MS <= 200);
}

// =============================================================================
// Pin Assignment Tests
// =============================================================================

#[test]
fn led_pin_defined() {
    assert!(!pins::LED_STATUS.is_empty());
}

#[test]
fn i2c_pins_defined() {
    assert!(!pins::I2C1_SCL.is_empty());
    assert!(!pins::I2C1_SDA.is_empty());
}

#[test]
fn encoder_pins_defined() {
    assert!(!pins::ENCODER_A.is_empty());
    assert!(!pins::ENCODER_B.is_empty());
    assert!(!pins::ENCODER_SW.is_empty());
}

#[test]
fn ptt_pin_defined() {
    assert!(!pins::PTT_IN.is_empty());
}

#[test]
fn tr_relay_pin_defined() {
    assert!(!pins::TR_RELAY.is_empty());
}

#[test]
fn lpf_select_pins_defined() {
    assert!(!pins::LPF_SEL0.is_empty());
    assert!(!pins::LPF_SEL1.is_empty());
    assert!(!pins::LPF_SEL2.is_empty());
}

#[test]
fn audio_pins_defined() {
    assert!(!pins::AUDIO_ADC.is_empty());
    assert!(!pins::AUDIO_DAC.is_empty());
}

#[test]
fn power_sensing_pins_defined() {
    assert!(!pins::FWD_POWER.is_empty());
    assert!(!pins::REF_POWER.is_empty());
}

#[test]
fn usb_pins_defined() {
    assert!(!pins::USB_DP.is_empty());
    assert!(!pins::USB_DM.is_empty());
}

#[test]
fn pa_pins_defined() {
    assert!(!pins::PA_DRIVE.is_empty());
    assert!(!pins::PA_AH.is_empty());
    assert!(!pins::PA_AL.is_empty());
    assert!(!pins::PA_BH.is_empty());
    assert!(!pins::PA_BL.is_empty());
}

// =============================================================================
// DMA Channel Tests
// =============================================================================

#[test]
fn dma_channels_unique() {
    // All DMA channels should be different
    let channels = [dma::I2C1_TX, dma::I2C1_RX, dma::ADC1, dma::DAC1, dma::ADC2];
    for i in 0..channels.len() {
        for j in (i + 1)..channels.len() {
            assert_ne!(channels[i], channels[j], "DMA channels must be unique");
        }
    }
}

// =============================================================================
// Timer Assignment Tests
// =============================================================================

#[test]
fn timer_channels_unique() {
    // All timer assignments should be different
    let timers = [timers::AUDIO_SAMPLE, timers::IQ_SAMPLE, timers::ENCODER,
                  timers::PA_PWM, timers::GENERAL];
    for i in 0..timers.len() {
        for j in (i + 1)..timers.len() {
            assert_ne!(timers[i], timers[j], "Timer assignments must be unique");
        }
    }
}

#[test]
fn pa_pwm_uses_advanced_timer() {
    // TIM1 or TIM8 are advanced timers with complementary outputs
    assert!(timers::PA_PWM == 1 || timers::PA_PWM == 8);
}
