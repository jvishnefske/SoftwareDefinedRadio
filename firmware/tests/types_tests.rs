//! Types Module Tests
//!
//! Tests for domain types (Frequency, Band, Mode, etc.)
//! Run with: cargo test --target x86_64-unknown-linux-gnu --no-default-features --features std --test types_tests

use sdr_firmware::types::{Band, Frequency, Mode, PowerLevel, SwrReading, TuningStep, TxRxState};

// =============================================================================
// Frequency Tests
// =============================================================================

#[test]
fn test_frequency_from_hz_valid() {
    // Valid frequencies
    assert!(Frequency::from_hz(7_000_000).is_some());
    assert!(Frequency::from_hz(3_500_000).is_some()); // Min
    assert!(Frequency::from_hz(21_450_000).is_some()); // Max
    assert!(Frequency::from_hz(14_074_000).is_some()); // FT8 on 20m
}

#[test]
fn test_frequency_from_hz_invalid() {
    // Below minimum
    assert!(Frequency::from_hz(1_000_000).is_none());
    assert!(Frequency::from_hz(3_499_999).is_none());

    // Above maximum
    assert!(Frequency::from_hz(30_000_000).is_none());
    assert!(Frequency::from_hz(21_450_001).is_none());
}

#[test]
fn test_frequency_from_khz() {
    let freq = Frequency::from_khz(7074).unwrap();
    assert_eq!(freq.as_hz(), 7_074_000);
    assert_eq!(freq.as_khz(), 7074);
}

#[test]
fn test_frequency_as_4x_lo() {
    let freq = Frequency::from_hz(7_000_000).unwrap();
    assert_eq!(freq.as_4x_lo(), 28_000_000);
}

#[test]
fn test_frequency_tune_up() {
    let freq = Frequency::from_hz(7_000_000).unwrap();
    let tuned = freq.tune_up(TuningStep::KHz1);
    assert_eq!(tuned.as_hz(), 7_001_000);
}

#[test]
fn test_frequency_tune_down() {
    let freq = Frequency::from_hz(7_001_000).unwrap();
    let tuned = freq.tune_down(TuningStep::KHz1);
    assert_eq!(tuned.as_hz(), 7_000_000);
}

#[test]
fn test_frequency_tune_clamps_at_max() {
    let freq = Frequency::from_hz(21_449_000).unwrap();
    let tuned = freq.tune_up(TuningStep::MHz1);
    assert_eq!(tuned.as_hz(), Frequency::MAX_HZ);
}

#[test]
fn test_frequency_tune_clamps_at_min() {
    let freq = Frequency::from_hz(3_501_000).unwrap();
    let tuned = freq.tune_down(TuningStep::MHz1);
    assert_eq!(tuned.as_hz(), Frequency::MIN_HZ);
}

#[test]
fn test_frequency_as_mhz_f32() {
    let freq = Frequency::from_hz(7_074_000).unwrap();
    let mhz = freq.as_mhz_f32();
    assert!((mhz - 7.074).abs() < 0.0001);
}

// =============================================================================
// TuningStep Tests
// =============================================================================

#[test]
fn test_tuning_step_as_hz() {
    assert_eq!(TuningStep::Hz1.as_hz(), 1);
    assert_eq!(TuningStep::Hz10.as_hz(), 10);
    assert_eq!(TuningStep::Hz100.as_hz(), 100);
    assert_eq!(TuningStep::KHz1.as_hz(), 1_000);
    assert_eq!(TuningStep::KHz10.as_hz(), 10_000);
    assert_eq!(TuningStep::KHz100.as_hz(), 100_000);
    assert_eq!(TuningStep::MHz1.as_hz(), 1_000_000);
}

#[test]
fn test_tuning_step_next_larger() {
    assert_eq!(TuningStep::Hz1.next_larger(), TuningStep::Hz10);
    assert_eq!(TuningStep::KHz10.next_larger(), TuningStep::KHz100);
    assert_eq!(TuningStep::MHz1.next_larger(), TuningStep::Hz1); // Wraps
}

#[test]
fn test_tuning_step_next_smaller() {
    assert_eq!(TuningStep::Hz10.next_smaller(), TuningStep::Hz1);
    assert_eq!(TuningStep::KHz100.next_smaller(), TuningStep::KHz10);
    assert_eq!(TuningStep::Hz1.next_smaller(), TuningStep::MHz1); // Wraps
}

// =============================================================================
// Mode Tests
// =============================================================================

#[test]
fn test_mode_bandwidth() {
    assert_eq!(Mode::Lsb.bandwidth_hz(), 2700);
    assert_eq!(Mode::Usb.bandwidth_hz(), 2700);
    assert_eq!(Mode::Cw.bandwidth_hz(), 500);
    assert_eq!(Mode::CwR.bandwidth_hz(), 500);
    assert_eq!(Mode::Am.bandwidth_hz(), 6000);
    assert_eq!(Mode::Fm.bandwidth_hz(), 12000);
}

#[test]
fn test_mode_bfo_offset() {
    assert_eq!(Mode::Lsb.bfo_offset_hz(), 1500);
    assert_eq!(Mode::Usb.bfo_offset_hz(), -1500);
    assert_eq!(Mode::Cw.bfo_offset_hz(), -700);
    assert_eq!(Mode::CwR.bfo_offset_hz(), 700);
    assert_eq!(Mode::Am.bfo_offset_hz(), 0);
    assert_eq!(Mode::Fm.bfo_offset_hz(), 0);
}

#[test]
fn test_mode_inverted_sideband() {
    assert!(Mode::Lsb.inverted_sideband());
    assert!(!Mode::Usb.inverted_sideband());
    assert!(!Mode::Cw.inverted_sideband());
    assert!(Mode::CwR.inverted_sideband());
    assert!(!Mode::Am.inverted_sideband());
    assert!(!Mode::Fm.inverted_sideband());
}

#[test]
fn test_mode_default() {
    assert_eq!(Mode::default(), Mode::Lsb);
}

// =============================================================================
// Band Tests
// =============================================================================

#[test]
fn test_band_from_frequency() {
    // 80m
    let freq = Frequency::from_hz(3_600_000).unwrap();
    assert_eq!(Band::from_frequency(freq), Some(Band::M80));

    // 40m
    let freq = Frequency::from_hz(7_074_000).unwrap();
    assert_eq!(Band::from_frequency(freq), Some(Band::M40));

    // 30m
    let freq = Frequency::from_hz(10_136_000).unwrap();
    assert_eq!(Band::from_frequency(freq), Some(Band::M30));

    // 20m
    let freq = Frequency::from_hz(14_074_000).unwrap();
    assert_eq!(Band::from_frequency(freq), Some(Band::M20));

    // 17m
    let freq = Frequency::from_hz(18_100_000).unwrap();
    assert_eq!(Band::from_frequency(freq), Some(Band::M17));

    // 15m
    let freq = Frequency::from_hz(21_074_000).unwrap();
    assert_eq!(Band::from_frequency(freq), Some(Band::M15));
}

#[test]
fn test_band_from_frequency_out_of_band() {
    // Between bands
    let freq = Frequency::from_hz(5_000_000).unwrap();
    assert_eq!(Band::from_frequency(freq), None);
}

#[test]
fn test_band_start_end() {
    assert_eq!(Band::M80.start_hz(), 3_500_000);
    assert_eq!(Band::M80.end_hz(), 4_000_000);

    assert_eq!(Band::M40.start_hz(), 7_000_000);
    assert_eq!(Band::M40.end_hz(), 7_300_000);
}

#[test]
fn test_band_lpf_index() {
    assert_eq!(Band::M80.lpf_index(), 0);
    assert_eq!(Band::M40.lpf_index(), 1);
    assert_eq!(Band::M30.lpf_index(), 2);
    assert_eq!(Band::M20.lpf_index(), 2);
    assert_eq!(Band::M17.lpf_index(), 3);
    assert_eq!(Band::M15.lpf_index(), 4);
}

#[test]
fn test_band_default_mode() {
    // Lower bands default to LSB
    assert_eq!(Band::M80.default_mode(), Mode::Lsb);
    assert_eq!(Band::M40.default_mode(), Mode::Lsb);

    // Higher bands default to USB
    assert_eq!(Band::M30.default_mode(), Mode::Usb);
    assert_eq!(Band::M20.default_mode(), Mode::Usb);
    assert_eq!(Band::M17.default_mode(), Mode::Usb);
    assert_eq!(Band::M15.default_mode(), Mode::Usb);
}

// =============================================================================
// PowerLevel Tests
// =============================================================================

#[test]
fn test_power_level_from_percent() {
    let power = PowerLevel::from_percent(50);
    assert_eq!(power.as_percent(), 50);
}

#[test]
fn test_power_level_clamps_at_100() {
    let power = PowerLevel::from_percent(150);
    assert_eq!(power.as_percent(), 100);
}

#[test]
fn test_power_level_pwm_duty() {
    let power = PowerLevel::from_percent(100);
    assert_eq!(power.as_pwm_duty(), 65500);

    let power = PowerLevel::from_percent(50);
    assert_eq!(power.as_pwm_duty(), 32750);

    let power = PowerLevel::from_percent(0);
    assert_eq!(power.as_pwm_duty(), 0);
}

#[test]
fn test_power_level_default() {
    let power = PowerLevel::default();
    assert_eq!(power.as_percent(), 50);
}

#[test]
fn test_power_level_constants() {
    assert_eq!(PowerLevel::MIN.as_percent(), 0);
    assert_eq!(PowerLevel::MAX.as_percent(), 100);
}

// =============================================================================
// SwrReading Tests
// =============================================================================

#[test]
fn test_swr_perfect_match() {
    let reading = SwrReading {
        forward: 100,
        reflected: 0,
    };
    let swr = reading.swr_ratio();
    assert!(swr < 1.1, "Expected SWR ~1.0, got {}", swr);
}

#[test]
fn test_swr_high_reflection() {
    let reading = SwrReading {
        forward: 100,
        reflected: 50,
    };
    let swr = reading.swr_ratio();
    // rho = sqrt(50/100) = 0.707
    // SWR = (1 + 0.707) / (1 - 0.707) ≈ 5.83
    assert!(swr > 5.0 && swr < 6.0, "Expected SWR ~5.83, got {}", swr);
}

#[test]
fn test_swr_no_forward() {
    let reading = SwrReading {
        forward: 0,
        reflected: 100,
    };
    let swr = reading.swr_ratio();
    assert_eq!(swr, 999.0);
}

#[test]
fn test_swr_is_acceptable() {
    // Good match
    let reading = SwrReading {
        forward: 100,
        reflected: 0,
    };
    assert!(reading.is_acceptable());

    // Poor match (high reflected)
    let reading = SwrReading {
        forward: 100,
        reflected: 80,
    };
    assert!(!reading.is_acceptable());
}

// =============================================================================
// TxRxState Tests
// =============================================================================

#[test]
fn test_txrx_state_default() {
    assert_eq!(TxRxState::default(), TxRxState::Rx);
}

#[test]
fn test_txrx_state_equality() {
    assert_eq!(TxRxState::Tx, TxRxState::Tx);
    assert_ne!(TxRxState::Tx, TxRxState::Rx);
}

// =============================================================================
// Band Edge and Frequency Boundary Tests
// =============================================================================

#[test]
fn test_band_edge_frequencies_80m() {
    // At band edges
    let low_edge = Frequency::from_hz(3_500_000).unwrap();
    let high_edge = Frequency::from_hz(3_999_999).unwrap();

    assert_eq!(Band::from_frequency(low_edge), Some(Band::M80));
    assert_eq!(Band::from_frequency(high_edge), Some(Band::M80));
}

#[test]
fn test_band_edge_frequencies_40m() {
    let low_edge = Frequency::from_hz(7_000_000).unwrap();
    let high_edge = Frequency::from_hz(7_299_999).unwrap();

    assert_eq!(Band::from_frequency(low_edge), Some(Band::M40));
    assert_eq!(Band::from_frequency(high_edge), Some(Band::M40));
}

#[test]
fn test_band_edge_frequencies_20m() {
    let low_edge = Frequency::from_hz(14_000_000).unwrap();
    let high_edge = Frequency::from_hz(14_349_999).unwrap();

    assert_eq!(Band::from_frequency(low_edge), Some(Band::M20));
    assert_eq!(Band::from_frequency(high_edge), Some(Band::M20));
}

#[test]
fn test_frequency_at_band_boundary() {
    // Just above 40m should not be in 40m band
    let above_40m = Frequency::from_hz(7_300_001);
    if let Some(freq) = above_40m {
        assert_ne!(Band::from_frequency(freq), Some(Band::M40));
    }
}

#[test]
fn test_frequency_popular_qrg() {
    // FT8 frequencies
    let ft8_80m = Frequency::from_hz(3_573_000).unwrap();
    let ft8_40m = Frequency::from_hz(7_074_000).unwrap();
    let ft8_20m = Frequency::from_hz(14_074_000).unwrap();

    assert_eq!(Band::from_frequency(ft8_80m), Some(Band::M80));
    assert_eq!(Band::from_frequency(ft8_40m), Some(Band::M40));
    assert_eq!(Band::from_frequency(ft8_20m), Some(Band::M20));
}

#[test]
fn test_frequency_cw_qrg() {
    // CW portions of bands
    let cw_40m = Frequency::from_hz(7_030_000).unwrap();
    let cw_20m = Frequency::from_hz(14_040_000).unwrap();

    assert_eq!(Band::from_frequency(cw_40m), Some(Band::M40));
    assert_eq!(Band::from_frequency(cw_20m), Some(Band::M20));
}

// =============================================================================
// Mode Audio Offset Tests (BFO)
// =============================================================================

#[test]
fn test_mode_bfo_offset_cw() {
    // CW typically uses 700-800 Hz offset (can be negative for reversed sideband)
    let offset = Mode::Cw.bfo_offset_hz();
    assert!(offset.abs() > 600 && offset.abs() < 900, "CW offset should be ~700Hz, got {}", offset);
}

#[test]
fn test_mode_bfo_offset_ssb() {
    // SSB uses sideband offset (typically 1500 Hz center)
    let lsb_offset = Mode::Lsb.bfo_offset_hz();
    let usb_offset = Mode::Usb.bfo_offset_hz();

    // LSB and USB should have opposite sign offsets
    assert!(lsb_offset != usb_offset, "LSB and USB should have different offsets");
}

#[test]
fn test_mode_bandwidth_cw() {
    let bw = Mode::Cw.bandwidth_hz();
    // CW filter typically 400-800 Hz
    assert!(bw >= 300 && bw <= 1000, "CW bandwidth should be ~500Hz, got {}", bw);
}

#[test]
fn test_mode_bandwidth_ssb() {
    let bw_lsb = Mode::Lsb.bandwidth_hz();
    let bw_usb = Mode::Usb.bandwidth_hz();

    // SSB typically 2.4-3 kHz
    assert!(bw_lsb >= 2000 && bw_lsb <= 3500, "LSB bandwidth should be ~2.7kHz, got {}", bw_lsb);
    assert_eq!(bw_lsb, bw_usb, "LSB and USB should have same bandwidth");
}

#[test]
fn test_mode_bandwidth_am() {
    let bw = Mode::Am.bandwidth_hz();
    // AM typically 6-9 kHz (both sidebands)
    assert!(bw >= 5000 && bw <= 10000, "AM bandwidth should be ~6kHz, got {}", bw);
}

// =============================================================================
// Power Level Edge Cases
// =============================================================================

#[test]
fn test_power_level_zero() {
    let power = PowerLevel::from_percent(0);
    assert_eq!(power.as_percent(), 0);
    assert_eq!(power.as_pwm_duty(), 0);
}

#[test]
fn test_power_level_qrp() {
    // QRP power level (5W = ~10% of 50W max)
    let power = PowerLevel::from_percent(10);
    assert_eq!(power.as_percent(), 10);
    assert!(power.as_pwm_duty() > 0);
}

// =============================================================================
// SWR Protection Thresholds
// =============================================================================

#[test]
fn test_swr_threshold_acceptable() {
    // SWR <= 3:1 is generally acceptable
    let reading = SwrReading {
        forward: 100,
        reflected: 10, // Low reflection = good match
    };
    assert!(reading.is_acceptable());
}

#[test]
fn test_swr_marginal() {
    // SWR around 3:1 - higher reflected power
    let reading = SwrReading {
        forward: 100,
        reflected: 25, // Higher reflection
    };
    // Should have measurable SWR
    let swr = reading.swr_ratio();
    assert!(swr > 1.5 && swr < 5.0, "Expected moderate SWR, got {}", swr);
}
