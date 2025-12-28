//! Oscillator Module Tests
//!
//! Tests for digital oscillators (sine, quadrature, NCO)
//! Run with: cargo test --target x86_64-unknown-linux-gnu --no-default-features --features std --test oscillator_tests

use sdr_firmware::dsp::oscillator::{Nco, QuadratureOscillator, SineOscillator};

const EPSILON: f32 = 0.01;

// =============================================================================
// SineOscillator Tests
// =============================================================================

#[test]
fn test_sine_oscillator_creation() {
    let osc = SineOscillator::new();
    assert_eq!(osc.phase(), 0.0);
}

#[test]
fn test_sine_oscillator_frequency() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // At 1kHz and 48kHz sample rate, period is 48 samples
    // After 12 samples (1/4 period), should be at peak
    for _ in 0..12 {
        osc.next();
    }
    let sample = osc.next();
    assert!(
        sample > 0.8,
        "Expected near-peak after 1/4 period, got {}",
        sample
    );
}

#[test]
fn test_sine_oscillator_full_period() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // After one full period (48 samples), should be back near 0
    for _ in 0..48 {
        osc.next();
    }
    let sample = osc.next();
    assert!(
        sample.abs() < 0.2,
        "Expected near-zero after full period, got {}",
        sample
    );
}

#[test]
fn test_sine_oscillator_range() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // All samples should be in range [-1, 1]
    for _ in 0..1000 {
        let sample = osc.next();
        assert!(
            sample >= -1.0 && sample <= 1.0,
            "Sample out of range: {}",
            sample
        );
    }
}

#[test]
fn test_sine_oscillator_set_frequency() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // Process some samples
    for _ in 0..10 {
        osc.next();
    }

    // Change frequency
    osc.set_frequency(2000.0, 48000.0);

    // Verify it still produces valid samples
    for _ in 0..100 {
        let sample = osc.next();
        assert!(sample.is_finite());
    }
}

#[test]
fn test_sine_oscillator_reset() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // Advance oscillator
    for _ in 0..25 {
        osc.next();
    }

    // Reset
    osc.reset();

    // Should be back at phase 0
    assert_eq!(osc.phase(), 0.0);
}

#[test]
fn test_sine_oscillator_with_offset() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // With 0.25 offset (90 degrees), first sample should be near 1.0 (cosine)
    let sample = osc.next_with_offset(0.25);
    assert!(
        (sample - 1.0).abs() < EPSILON,
        "With 90° offset, expected ~1.0, got {}",
        sample
    );
}

// =============================================================================
// QuadratureOscillator Tests
// =============================================================================

#[test]
fn test_quadrature_oscillator_creation() {
    let mut osc = QuadratureOscillator::new();
    // Get initial output by calling next() - starts at (1, 0)
    let (i, q) = osc.next();

    // Initial I should be 1 (cosine at 0)
    // Initial Q should be 0 (sine at 0)
    assert!(
        (i - 1.0).abs() < EPSILON,
        "Initial I should be 1, got {}",
        i
    );
    assert!(q.abs() < EPSILON, "Initial Q should be 0, got {}", q);
}

#[test]
fn test_quadrature_90_degree_relationship() {
    let mut osc = QuadratureOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // I and Q should maintain 90 degree phase relationship
    for _ in 0..100 {
        let (i, q) = osc.next();

        // i^2 + q^2 should always be close to 1 (unit circle)
        let magnitude = (i * i + q * q).sqrt();
        assert!(
            (magnitude - 1.0).abs() < 0.1,
            "Magnitude should be ~1, got {}",
            magnitude
        );
    }
}

#[test]
fn test_quadrature_range() {
    let mut osc = QuadratureOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    for _ in 0..1000 {
        let (i, q) = osc.next();
        assert!(
            i >= -1.1 && i <= 1.1,
            "I component out of range: {}",
            i
        );
        assert!(
            q >= -1.1 && q <= 1.1,
            "Q component out of range: {}",
            q
        );
    }
}

#[test]
fn test_quadrature_reset() {
    let mut osc = QuadratureOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // Advance
    for _ in 0..50 {
        osc.next();
    }

    // Reset
    osc.reset();

    let (i, q) = osc.next();
    assert!(
        (i - 1.0).abs() < EPSILON,
        "After reset I should be 1, got {}",
        i
    );
    assert!(
        q.abs() < EPSILON,
        "After reset Q should be 0, got {}",
        q
    );
}

// =============================================================================
// NCO Tests
// =============================================================================

#[test]
fn test_nco_creation() {
    let mut nco = Nco::new();
    let (cos, sin) = nco.next_iq();

    // Initial: sin(0) = 0, cos(0) = 1
    assert!(sin.abs() < EPSILON, "Initial sin should be 0, got {}", sin);
    assert!(
        (cos - 1.0).abs() < EPSILON,
        "Initial cos should be 1, got {}",
        cos
    );
}

#[test]
fn test_nco_frequency_update() {
    let mut nco = Nco::new();
    nco.set_frequency(1000, 48000);

    // Process some samples
    for _ in 0..10 {
        nco.next();
    }

    // Update frequency
    nco.set_frequency(2000, 48000);

    // Should continue producing valid samples
    for _ in 0..100 {
        let sample = nco.next();
        assert!(sample.is_finite());
    }
}

#[test]
fn test_nco_phase_accumulator_wraps() {
    let mut nco = Nco::new();
    nco.set_frequency(10000, 48000);

    // Run for many samples to ensure phase wraps correctly
    for _ in 0..10000 {
        let sample = nco.next();
        assert!(
            sample >= -1.1 && sample <= 1.1,
            "Sample out of range: {}",
            sample
        );
    }
}

#[test]
fn test_nco_orthogonality() {
    let mut nco = Nco::new();
    nco.set_frequency(1000, 48000);

    for _ in 0..100 {
        let (cos, sin) = nco.next_iq();

        // sin^2 + cos^2 should be close to 1
        let sum_sq = sin * sin + cos * cos;
        assert!(
            (sum_sq - 1.0).abs() < 0.1,
            "sin^2 + cos^2 should be ~1, got {}",
            sum_sq
        );
    }
}

#[test]
fn test_nco_reset() {
    let mut nco = Nco::new();
    nco.set_frequency(1000, 48000);

    // Advance
    for _ in 0..100 {
        nco.next();
    }

    // Reset
    nco.reset();

    let (cos, sin) = nco.next_iq();
    assert!(
        sin.abs() < EPSILON,
        "After reset sin should be 0, got {}",
        sin
    );
    assert!(
        (cos - 1.0).abs() < EPSILON,
        "After reset cos should be 1, got {}",
        cos
    );
}

#[test]
fn test_nco_set_frequency_f32() {
    let mut nco = Nco::new();
    nco.set_frequency_f32(1000.5, 48000.0);

    // Should produce valid samples
    for _ in 0..100 {
        let sample = nco.next();
        assert!(sample.is_finite());
        assert!(sample >= -1.0 && sample <= 1.0);
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_very_low_frequency() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(1.0, 48000.0);

    // Should still produce valid samples
    for _ in 0..1000 {
        let sample = osc.next();
        assert!(sample.is_finite());
        assert!(sample >= -1.0 && sample <= 1.0);
    }
}

#[test]
fn test_high_frequency_near_nyquist() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(20000.0, 48000.0);

    // Should still produce valid samples
    for _ in 0..1000 {
        let sample = osc.next();
        assert!(sample.is_finite());
        assert!(sample >= -1.0 && sample <= 1.0);
    }
}

#[test]
fn test_zero_frequency() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(0.0, 48000.0);

    // DC - should always be 0 (sin of constant phase 0)
    for _ in 0..100 {
        let sample = osc.next();
        assert!(sample.abs() < EPSILON, "Zero freq should output 0, got {}", sample);
    }
}

// =============================================================================
// Frequency Accuracy Tests
// =============================================================================

#[test]
fn test_frequency_accuracy_1khz() {
    let mut osc = SineOscillator::new();
    let sample_rate = 48000.0;
    let freq = 1000.0;
    osc.set_frequency(freq, sample_rate);

    // Count zero crossings over multiple periods
    let samples_per_period = (sample_rate / freq) as usize;
    let num_periods = 10;
    let total_samples = samples_per_period * num_periods;

    let mut prev_sample = 0.0;
    let mut zero_crossings = 0;

    for _ in 0..total_samples {
        let sample = osc.next();
        if prev_sample < 0.0 && sample >= 0.0 {
            zero_crossings += 1;
        }
        prev_sample = sample;
    }

    // Should have approximately num_periods zero crossings (positive going)
    assert!(
        (zero_crossings as i32 - num_periods as i32).abs() <= 1,
        "Expected ~{} zero crossings, got {}",
        num_periods,
        zero_crossings
    );
}

#[test]
fn test_frequency_accuracy_ft8() {
    // FT8 audio tone is typically around 1500 Hz
    let mut osc = SineOscillator::new();
    let sample_rate = 48000.0;
    let freq = 1500.0;
    osc.set_frequency(freq, sample_rate);

    // Measure period by finding peaks
    let samples_per_period = (sample_rate / freq) as usize; // 32 samples
    let mut max_sample = -1.0f32;
    let mut max_idx = 0;

    for i in 0..samples_per_period * 2 {
        let sample = osc.next();
        if sample > max_sample {
            max_sample = sample;
            max_idx = i;
        }
    }

    // First peak should be at 1/4 period
    let expected_first_peak = samples_per_period / 4;
    assert!(
        (max_idx as i32 - expected_first_peak as i32).abs() <= 2,
        "Peak at wrong position: expected ~{}, got {}",
        expected_first_peak,
        max_idx
    );
}

#[test]
fn test_quadrature_phase_accuracy() {
    let mut osc = QuadratureOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // After 1/4 period (12 samples at 1kHz/48kHz), I and Q should swap roles
    // I starts at 1 (cos), Q starts at 0 (sin)
    // After 90 degrees: I should be ~0, Q should be ~1

    let samples_per_quarter = 12;
    for _ in 0..samples_per_quarter {
        osc.next();
    }

    let (i, q) = osc.next();
    assert!(
        i.abs() < 0.2,
        "After 90°, I should be ~0, got {}",
        i
    );
    assert!(
        (q - 1.0).abs() < 0.2 || (q + 1.0).abs() < 0.2,
        "After 90°, Q should be ~±1, got {}",
        q
    );
}

#[test]
fn test_nco_frequency_precision() {
    let mut nco = Nco::new();
    let sample_rate = 48000;
    let freq = 7074; // FT8 frequency offset in Hz

    nco.set_frequency(freq, sample_rate);

    // Generate one second of samples and measure actual frequency
    let one_second = sample_rate;
    let mut zero_crossings = 0;
    let mut prev = 0.0;

    for _ in 0..one_second {
        let sample = nco.next();
        if prev < 0.0 && sample >= 0.0 {
            zero_crossings += 1;
        }
        prev = sample;
    }

    // Frequency = zero crossings per second
    let measured_freq = zero_crossings;
    let error = (measured_freq as i32 - freq as i32).abs();

    assert!(
        error <= 1,
        "Frequency error too high: expected {}, got {}, error {}",
        freq,
        measured_freq,
        error
    );
}

#[test]
fn test_nco_phase_continuity() {
    let mut nco = Nco::new();
    nco.set_frequency(1000, 48000);

    // Samples should change smoothly without jumps
    let mut prev = nco.next();
    for _ in 0..1000 {
        let curr = nco.next();
        let delta = (curr - prev).abs();

        // Maximum step size depends on frequency
        // At 1kHz/48kHz, phase increment is ~0.131 rad, max sample delta ~0.13
        assert!(
            delta < 0.5,
            "Phase discontinuity: prev={}, curr={}, delta={}",
            prev,
            curr,
            delta
        );
        prev = curr;
    }
}

#[test]
fn test_sine_energy_conservation() {
    let mut osc = SineOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // RMS of sine wave should be 1/sqrt(2) ≈ 0.707
    let mut sum_sq = 0.0;
    let samples = 4800; // 100 periods

    for _ in 0..samples {
        let s = osc.next();
        sum_sq += s * s;
    }

    let rms = (sum_sq / samples as f32).sqrt();
    let expected_rms = 1.0 / 2.0f32.sqrt();

    assert!(
        (rms - expected_rms).abs() < 0.05,
        "RMS should be ~0.707, got {}",
        rms
    );
}

#[test]
fn test_quadrature_orthogonality_integration() {
    let mut osc = QuadratureOscillator::new();
    osc.set_frequency(1000.0, 48000.0);

    // Integral of I*Q over complete periods should be near zero
    let samples_per_period = 48;
    let num_periods = 10;
    let mut integral = 0.0;

    for _ in 0..(samples_per_period * num_periods) {
        let (i, q) = osc.next();
        integral += i * q;
    }

    let normalized = integral / (samples_per_period * num_periods) as f32;
    assert!(
        normalized.abs() < 0.1,
        "I and Q should be orthogonal, integral={}",
        normalized
    );
}
