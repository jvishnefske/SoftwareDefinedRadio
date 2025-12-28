//! AGC Module Tests
//!
//! Tests for Automatic Gain Control
//! Run with: cargo test --target x86_64-unknown-linux-gnu --no-default-features --features std --test agc_tests

use sdr_firmware::dsp::agc::{Agc, AgcConfig, SMeter};

// =============================================================================
// AGC Configuration Tests
// =============================================================================

#[test]
fn test_agc_config_default() {
    let config = AgcConfig::default();
    assert!(config.target_level > 0.0 && config.target_level <= 1.0);
    assert!(config.attack_samples > 0);
    assert!(config.decay_samples > 0);
    assert!(config.decay_samples > config.attack_samples); // Fast attack, slow decay
}

#[test]
fn test_agc_config_from_ms() {
    let config = AgcConfig::from_ms(48000, 10, 500);
    assert_eq!(config.attack_samples, 480);  // 10ms at 48kHz
    assert_eq!(config.decay_samples, 24000); // 500ms at 48kHz
}

// =============================================================================
// AGC Processing Tests
// =============================================================================

#[test]
fn test_agc_creation() {
    let agc = Agc::new(AgcConfig::default());
    let db = agc.gain_db();
    assert!(db.is_finite(), "Initial gain_db should be finite");
}

#[test]
fn test_agc_default() {
    let agc = Agc::default();
    let db = agc.gain_db();
    assert!(db.is_finite());
}

#[test]
fn test_agc_boosts_weak_signals() {
    let mut agc = Agc::new(AgcConfig::default());

    // Process weak signal
    let weak_input = 0.01;
    let mut output = 0.0;
    for _ in 0..1000 {
        output = agc.process(weak_input);
    }

    // Output should be boosted closer to target level
    assert!(
        output > weak_input,
        "AGC should boost weak signals: input={}, output={}",
        weak_input,
        output
    );
}

#[test]
fn test_agc_output_finite() {
    let mut agc = Agc::new(AgcConfig::default());

    // Process various signal levels
    for level in [0.01, 0.1, 0.3, 0.5, 0.8] {
        for _ in 0..100 {
            let output = agc.process(level);
            assert!(
                output.is_finite(),
                "Output should be finite for input {}",
                level
            );
        }
    }
}

#[test]
fn test_agc_handles_zero_input() {
    let mut agc = Agc::new(AgcConfig::default());

    // Process zeros
    for _ in 0..100 {
        let output = agc.process(0.0);
        assert!(output.is_finite());
        assert_eq!(output, 0.0); // Zero in = zero out
    }
}

#[test]
fn test_agc_handles_negative_input() {
    let mut agc = Agc::new(AgcConfig::default());

    // Process negative samples
    for _ in 0..100 {
        let output = agc.process(-0.5);
        assert!(output.is_finite());
        assert!(output <= 0.0); // Should preserve sign
    }
}

#[test]
fn test_agc_fast_attack() {
    let config = AgcConfig {
        attack_samples: 10,
        decay_samples: 1000,
        target_level: 0.3,
        max_gain_db: 40.0,
        min_gain_db: -40.0,
        hang_samples: 50,
    };
    let mut agc = Agc::new(config);

    // Start with weak signal, then switch to strong
    for _ in 0..50 {
        agc.process(0.01);
    }
    let gain_db_before = agc.gain_db();

    // Strong signal should quickly reduce gain
    for _ in 0..50 {
        agc.process(0.9);
    }
    let gain_db_after = agc.gain_db();

    assert!(
        gain_db_after < gain_db_before,
        "Gain should decrease with strong signal: before={}, after={}",
        gain_db_before,
        gain_db_after
    );
}

#[test]
fn test_agc_reset() {
    let mut agc = Agc::new(AgcConfig::default());

    // Process to change state
    for _ in 0..100 {
        agc.process(0.9);
    }

    // Reset
    agc.reset();

    // Envelope should be reset
    assert_eq!(agc.envelope(), 0.0);
}

#[test]
fn test_agc_envelope_tracking() {
    let mut agc = Agc::new(AgcConfig::default());

    // Process signal
    for _ in 0..100 {
        agc.process(0.5);
    }

    let envelope = agc.envelope();
    assert!(envelope > 0.0, "Envelope should track signal");
}

#[test]
fn test_agc_process_block() {
    let mut agc = Agc::new(AgcConfig::default());

    let mut samples = [0.1, 0.2, 0.3, 0.4, 0.5];
    agc.process_block(&mut samples);

    for &s in &samples {
        assert!(s.is_finite());
    }
}

// =============================================================================
// S-Meter Tests
// =============================================================================

#[test]
fn test_smeter_creation() {
    let meter = SMeter::new();
    assert_eq!(meter.s_units(), 0);
    assert_eq!(meter.db_over_s9(), 0);
}

#[test]
fn test_smeter_default() {
    let meter = SMeter::default();
    assert_eq!(meter.s_units(), 0);
}

#[test]
fn test_smeter_update_from_agc() {
    let mut meter = SMeter::new();
    let mut agc = Agc::new(AgcConfig::default());

    // Process some signal through AGC
    for _ in 0..100 {
        agc.process(0.3);
    }

    // Update S-meter from AGC
    for _ in 0..100 {
        meter.update_from_agc(&agc);
    }

    // Should have some reading
    let value = meter.value();
    assert!(value >= 0.0, "S-meter value should be non-negative");
}

#[test]
fn test_smeter_update_from_level() {
    let mut meter = SMeter::new();

    // Feed in a moderate signal
    for _ in 0..100 {
        meter.update_from_level(0.1);
    }

    let s = meter.s_units();
    assert!(s <= 9, "S-units should be 0-9, got {}", s);
}

#[test]
fn test_smeter_strong_signal() {
    let mut meter = SMeter::new();

    // Strong signal
    for _ in 0..1000 {
        meter.update_from_level(0.9);
    }

    // Should read high
    let s = meter.s_units();
    assert!(s >= 5, "Strong signal should read high S-units, got {}", s);
}

#[test]
fn test_smeter_weak_signal() {
    let mut meter = SMeter::new();

    // Very weak signal
    for _ in 0..1000 {
        meter.update_from_level(0.001);
    }

    // Should read low
    let s = meter.s_units();
    assert!(s <= 4, "Weak signal should read low S-units, got {}", s);
}

#[test]
fn test_smeter_as_percent() {
    let mut meter = SMeter::new();

    for _ in 0..100 {
        meter.update_from_level(0.3);
    }

    let percent = meter.as_percent();
    assert!(
        percent <= 100,
        "Percent should be 0-100, got {}",
        percent
    );
}

// =============================================================================
// S-Meter Calibration Tests (IARU standard: 6 dB per S-unit)
// =============================================================================

#[test]
fn test_smeter_6db_per_s_unit() {
    // IARU standard: S9 = -73 dBm, each S-unit = 6 dB
    // We verify the relative relationship: signal amplitude ratio
    let mut meter1 = SMeter::new();
    let mut meter2 = SMeter::new();

    // Level ratio of 2:1 = 6 dB = 1 S-unit difference
    let level_high = 0.2;
    let level_low = 0.1; // Half amplitude = -6 dB

    for _ in 0..1000 {
        meter1.update_from_level(level_high);
        meter2.update_from_level(level_low);
    }

    let s_high = meter1.value();
    let s_low = meter2.value();

    // Difference should be approximately 1 S-unit (6 dB)
    let diff = s_high - s_low;
    assert!(
        diff > 0.8 && diff < 1.5,
        "2:1 amplitude ratio should be ~1 S-unit, got {} diff (high={}, low={})",
        diff,
        s_high,
        s_low
    );
}

#[test]
fn test_smeter_db_over_s9_granularity() {
    let mut meter = SMeter::new();

    // Very strong signal above S9
    for _ in 0..1000 {
        meter.update_from_level(1.0);
    }

    // Should be S9 + some dB
    let s = meter.s_units();
    let db_over = meter.db_over_s9();

    assert_eq!(s, 9, "Strong signal should read S9, got S{}", s);
    assert!(db_over > 0, "Strong signal should have dB over S9, got {}", db_over);
}

#[test]
fn test_smeter_s9_threshold() {
    let mut meter = SMeter::new();

    // Find the level that gives exactly S9
    for _ in 0..1000 {
        meter.update_from_level(0.5);
    }

    let value = meter.value();
    // At S9 (value=9), db_over_s9 should be 0
    // At S9+ (value>9), db_over_s9 should be positive
    if value <= 9.0 {
        assert_eq!(meter.db_over_s9(), 0, "At or below S9, dB over should be 0");
    } else {
        assert!(meter.db_over_s9() > 0, "Above S9, dB over should be positive");
    }
}

#[test]
fn test_smeter_smoothing() {
    let mut meter = SMeter::new();

    // Abrupt level change
    for _ in 0..10 {
        meter.update_from_level(0.01);
    }
    let s_low = meter.value();

    // Single spike
    meter.update_from_level(0.9);
    let s_after_spike = meter.value();

    // Smoothing should prevent immediate jump
    assert!(
        s_after_spike < s_low + 5.0,
        "Smoothing should limit response to spike: low={}, after={}",
        s_low,
        s_after_spike
    );
}

#[test]
fn test_smeter_floor_at_zero() {
    let mut meter = SMeter::new();

    // Very weak/no signal
    for _ in 0..1000 {
        meter.update_from_level(0.00001);
    }

    let s = meter.s_units();
    assert!(s <= 1, "Noise floor should be S0-S1, got S{}", s);
}

#[test]
fn test_smeter_clamping_high() {
    let mut meter = SMeter::new();

    // Extremely strong signal
    for _ in 0..1000 {
        meter.update_from_level(10.0); // Beyond normal range
    }

    // Should clamp to max (S9+60 = value 19)
    let value = meter.value();
    assert!(value <= 15.0, "S-meter should clamp at max, got {}", value);
    assert!(value >= 9.0, "Strong signal should be at least S9, got {}", value);
}

#[test]
fn test_smeter_dynamic_range() {
    // Test meter response across full dynamic range
    let levels = [0.0001, 0.001, 0.01, 0.1, 0.5, 1.0];
    let mut readings = Vec::new();

    for &level in &levels {
        let mut meter = SMeter::new();
        for _ in 0..1000 {
            meter.update_from_level(level);
        }
        readings.push(meter.value());
    }

    // Readings should be monotonically increasing
    for i in 1..readings.len() {
        assert!(
            readings[i] >= readings[i - 1],
            "S-meter should increase with level: {} >= {} at index {}",
            readings[i],
            readings[i - 1],
            i
        );
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_agc_with_alternating_signal() {
    let mut agc = Agc::new(AgcConfig::default());

    // Alternating positive/negative
    for i in 0..1000 {
        let input = if i % 2 == 0 { 0.3 } else { -0.3 };
        let output = agc.process(input);
        assert!(output.is_finite());
    }
}

#[test]
fn test_agc_gain_stays_bounded() {
    let config = AgcConfig {
        attack_samples: 10,
        decay_samples: 100,
        target_level: 0.3,
        max_gain_db: 20.0,
        min_gain_db: -20.0,
        hang_samples: 10,
    };
    let mut agc = Agc::new(config);

    // Very weak signal - gain should not exceed max
    for _ in 0..10000 {
        agc.process(0.0001);
    }
    let db = agc.gain_db();
    assert!(db <= 20.0 + 1.0, "Gain should not exceed max: {}", db);

    // Very strong signal - gain should not go below min
    for _ in 0..10000 {
        agc.process(0.999);
    }
    let db = agc.gain_db();
    assert!(db >= -20.0 - 1.0, "Gain should not go below min: {}", db);
}
