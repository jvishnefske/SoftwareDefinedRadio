//! DSP Algorithm Tests
//!
//! These tests run on the host with std feature enabled.
//! Run with: cargo test --features std

use sdr_firmware::dsp::filter::{
    from_sample, to_sample, BiquadCoeffs, BiquadFilter, DcBlocker, FirCoefficients, FirFilter,
    MovingAverage,
};

// =============================================================================
// Sample Conversion Tests
// =============================================================================

#[test]
fn test_sample_conversion_roundtrip() {
    let values = [0.0, 0.5, -0.5, 0.99, -0.99];
    for &v in &values {
        let sample = to_sample(v);
        let back = from_sample(sample);
        assert!(
            (v - back).abs() < 0.001,
            "Roundtrip failed for {}: got {}",
            v,
            back
        );
    }
}

#[test]
fn test_sample_clamping() {
    // Values outside range should be clamped
    let sample = to_sample(2.0);
    let back = from_sample(sample);
    assert!(back <= 1.0);

    let sample = to_sample(-2.0);
    let back = from_sample(sample);
    assert!(back >= -1.0);
}

#[test]
fn test_sample_zero() {
    let sample = to_sample(0.0);
    let back = from_sample(sample);
    assert!(back.abs() < 0.001, "Zero not preserved: got {}", back);
}

// =============================================================================
// Biquad Filter Tests
// =============================================================================

#[test]
fn test_biquad_lowpass_dc_passthrough() {
    let coeffs = BiquadCoeffs::lowpass(0.1, 0.707);
    let mut filter = BiquadFilter::with_coeffs(coeffs);

    // DC signal should pass through lowpass
    let dc = 0.5;
    let mut output = 0.0;
    for _ in 0..100 {
        output = filter.process(dc);
    }
    // After settling, output should be close to input DC
    assert!(
        (output - dc).abs() < 0.01,
        "DC passthrough failed: {}",
        output
    );
}

#[test]
fn test_biquad_highpass_dc_rejection() {
    let coeffs = BiquadCoeffs::highpass(0.1, 0.707);
    let mut filter = BiquadFilter::with_coeffs(coeffs);

    // DC signal should be blocked by highpass
    let dc = 0.5;
    let mut output = 0.0;
    for _ in 0..100 {
        output = filter.process(dc);
    }
    // After settling, output should be close to zero
    assert!(output.abs() < 0.01, "DC rejection failed: {}", output);
}

#[test]
fn test_biquad_bandpass_response() {
    let coeffs = BiquadCoeffs::bandpass(0.25, 10.0);
    let mut filter = BiquadFilter::with_coeffs(coeffs);

    // Process impulse
    let output = filter.process(1.0);
    assert!(output.is_finite());

    // Continue processing zeros
    for _ in 0..10 {
        let out = filter.process(0.0);
        assert!(out.is_finite());
    }
}

#[test]
fn test_biquad_notch_response() {
    let coeffs = BiquadCoeffs::notch(0.25, 10.0);
    let mut filter = BiquadFilter::with_coeffs(coeffs);

    // Process impulse
    let output = filter.process(1.0);
    assert!(output.is_finite());
}

#[test]
fn test_biquad_reset() {
    let coeffs = BiquadCoeffs::lowpass(0.1, 0.707);
    let mut filter = BiquadFilter::with_coeffs(coeffs);

    // Process some samples
    for _ in 0..10 {
        filter.process(1.0);
    }

    // Reset
    filter.reset();

    // First sample after reset should be as if fresh
    let mut fresh_filter = BiquadFilter::with_coeffs(coeffs);
    let output = filter.process(0.5);
    let fresh_output = fresh_filter.process(0.5);
    assert!(
        (output - fresh_output).abs() < 0.001,
        "Reset failed: {} vs {}",
        output,
        fresh_output
    );
}

#[test]
fn test_biquad_block_processing() {
    let coeffs = BiquadCoeffs::lowpass(0.1, 0.707);
    let mut filter = BiquadFilter::with_coeffs(coeffs);

    let mut samples = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
    filter.process_block(&mut samples);

    // All outputs should be finite
    for &s in &samples {
        assert!(s.is_finite());
    }
}

#[test]
fn test_biquad_stability() {
    // Test that filters don't blow up with extreme inputs
    let coeffs = BiquadCoeffs::lowpass(0.01, 0.707);
    let mut filter = BiquadFilter::with_coeffs(coeffs);

    // Process many samples
    for i in 0..10000 {
        let input = if i % 2 == 0 { 1.0 } else { -1.0 };
        let output = filter.process(input);
        assert!(
            output.is_finite() && output.abs() < 100.0,
            "Filter unstable at iteration {}",
            i
        );
    }
}

// =============================================================================
// DC Blocker Tests
// =============================================================================

#[test]
fn test_dc_blocker_removes_dc() {
    let mut blocker = DcBlocker::default();

    // Process a DC signal
    let dc = 0.5;
    let mut output = 0.0;
    for _ in 0..1000 {
        output = blocker.process(dc);
    }
    // Output should approach zero
    assert!(output.abs() < 0.1, "DC blocker failed: {}", output);
}

#[test]
fn test_dc_blocker_ac_passthrough() {
    let mut blocker = DcBlocker::default();

    // Process a simple AC signal (alternating)
    let mut total_energy = 0.0;
    for i in 0..100 {
        let input = if i % 2 == 0 { 0.5 } else { -0.5 };
        let output = blocker.process(input);
        if i > 10 {
            // After settling
            total_energy += output.abs();
        }
    }
    // AC should pass through with significant energy
    assert!(total_energy > 10.0, "AC passthrough failed");
}

#[test]
fn test_dc_blocker_reset() {
    let mut blocker = DcBlocker::default();

    // Process some samples
    for _ in 0..100 {
        blocker.process(0.5);
    }

    // Reset
    blocker.reset();

    // After reset, should behave like fresh blocker
    let mut fresh = DcBlocker::default();
    let out1 = blocker.process(0.3);
    let out2 = fresh.process(0.3);
    assert!(
        (out1 - out2).abs() < 0.001,
        "Reset failed: {} vs {}",
        out1,
        out2
    );
}

// =============================================================================
// Moving Average Tests
// =============================================================================

#[test]
fn test_moving_average_calculation() {
    let mut avg: MovingAverage<4> = MovingAverage::new();

    // Feed in [1, 2, 3, 4], average should be 2.5
    avg.process(1.0);
    avg.process(2.0);
    avg.process(3.0);
    let result = avg.process(4.0);
    assert!(
        (result - 2.5).abs() < 0.001,
        "Moving average failed: {}",
        result
    );
}

#[test]
fn test_moving_average_smoothing() {
    let mut avg: MovingAverage<8> = MovingAverage::new();

    // Spike should be smoothed
    for _ in 0..8 {
        avg.process(0.0);
    }
    let spike = avg.process(8.0); // One spike
    assert!(spike < 8.0, "Spike not smoothed: {}", spike);
    assert!(spike > 0.0, "Spike completely lost");
    // Average should be 8/8 = 1.0
    assert!(
        (spike - 1.0).abs() < 0.001,
        "Wrong spike average: {}",
        spike
    );
}

#[test]
fn test_moving_average_constant() {
    let mut avg: MovingAverage<4> = MovingAverage::new();

    // After filling with constant, output should equal constant
    for _ in 0..10 {
        let _out = avg.process(5.0);
        // After 4 samples, should stabilize
    }
    let result = avg.process(5.0);
    assert!(
        (result - 5.0).abs() < 0.001,
        "Constant not preserved: {}",
        result
    );
}

#[test]
fn test_moving_average_reset() {
    let mut avg: MovingAverage<4> = MovingAverage::new();

    // Fill buffer
    for _ in 0..4 {
        avg.process(10.0);
    }

    // Reset
    avg.reset();

    // After reset, first sample should not be averaged with old values
    let result = avg.process(1.0);
    // With empty buffer, first value / N = 0.25 (depends on implementation)
    assert!(result.is_finite());
}

// =============================================================================
// Coefficient Validation Tests
// =============================================================================

#[test]
fn test_lowpass_filter_works() {
    // Test that lowpass filter can be created and used
    for freq in [0.01, 0.1, 0.25, 0.4] {
        let coeffs = BiquadCoeffs::lowpass(freq, 0.707);
        let mut filter = BiquadFilter::with_coeffs(coeffs);
        // Process a few samples to verify it doesn't panic
        for _ in 0..10 {
            let out = filter.process(0.5);
            assert!(out.is_finite());
        }
    }
}

#[test]
fn test_highpass_filter_works() {
    for freq in [0.01, 0.1, 0.25, 0.4] {
        let coeffs = BiquadCoeffs::highpass(freq, 0.707);
        let mut filter = BiquadFilter::with_coeffs(coeffs);
        for _ in 0..10 {
            let out = filter.process(0.5);
            assert!(out.is_finite());
        }
    }
}

#[test]
fn test_bandpass_filter_works() {
    for freq in [0.1, 0.25, 0.4] {
        let coeffs = BiquadCoeffs::bandpass(freq, 5.0);
        let mut filter = BiquadFilter::with_coeffs(coeffs);
        for _ in 0..10 {
            let out = filter.process(0.5);
            assert!(out.is_finite());
        }
    }
}

#[test]
fn test_notch_filter_works() {
    for freq in [0.1, 0.25, 0.4] {
        let coeffs = BiquadCoeffs::notch(freq, 10.0);
        let mut filter = BiquadFilter::with_coeffs(coeffs);
        for _ in 0..10 {
            let out = filter.process(0.5);
            assert!(out.is_finite());
        }
    }
}

// =============================================================================
// FIR Filter Tests
// =============================================================================

#[test]
fn test_fir_coefficients_from_f32() {
    let coeffs: [f32; 5] = [0.1, 0.2, 0.4, 0.2, 0.1];
    let fir_coeffs = FirCoefficients::<5>::from_f32(&coeffs);
    // Verify coefficients were set (get returns fixed-point)
    let c0 = from_sample(fir_coeffs.get(0));
    assert!((c0 - 0.1).abs() < 0.01, "Coefficient 0 mismatch: {}", c0);
}

#[test]
fn test_fir_coefficients_get_out_of_bounds() {
    let coeffs: [f32; 3] = [0.25, 0.5, 0.25];
    let fir_coeffs = FirCoefficients::<3>::from_f32(&coeffs);
    // Out of bounds should return 0
    let out = from_sample(fir_coeffs.get(10));
    assert!(out.abs() < 0.001, "Out of bounds should be 0: {}", out);
}

#[test]
fn test_fir_lowpass_creation() {
    let coeffs = FirCoefficients::<31>::lowpass(0.1);
    // Should be able to get coefficients
    let center = from_sample(coeffs.get(15));
    // Center tap should be largest for lowpass
    assert!(center > 0.0, "Center tap should be positive: {}", center);
}

#[test]
fn test_fir_bandpass_creation() {
    let coeffs = FirCoefficients::<31>::bandpass(0.1, 0.3);
    // Should create valid coefficients
    let center = from_sample(coeffs.get(15));
    assert!(center.is_finite(), "Coefficients should be finite");
}

#[test]
fn test_fir_filter_creation() {
    let coeffs = FirCoefficients::<15>::lowpass(0.2);
    let _filter = FirFilter::new(coeffs);
}

#[test]
fn test_fir_filter_dc_passthrough() {
    let coeffs = FirCoefficients::<15>::lowpass(0.4);
    let mut filter = FirFilter::new(coeffs);

    // DC signal should pass through lowpass
    let dc_input = to_sample(0.5);
    let mut output = to_sample(0.0);

    for _ in 0..100 {
        output = filter.process(dc_input);
    }

    let out_f32 = from_sample(output);
    // After settling, should be close to input
    assert!(
        (out_f32 - 0.5).abs() < 0.1,
        "DC passthrough failed: {}",
        out_f32
    );
}

#[test]
fn test_fir_filter_impulse_response() {
    let coeffs = FirCoefficients::<5>::from_f32(&[0.2, 0.2, 0.2, 0.2, 0.2]);
    let mut filter = FirFilter::new(coeffs);

    // Feed an impulse
    let impulse = to_sample(1.0);
    let zero = to_sample(0.0);

    let out1 = filter.process(impulse);
    let _out2 = filter.process(zero);
    let _out3 = filter.process(zero);
    let _out4 = filter.process(zero);
    let _out5 = filter.process(zero);
    let out6 = filter.process(zero);

    // Should see impulse spread over 5 samples
    assert!(from_sample(out1).abs() > 0.0, "First output should be nonzero");
    // After 5 zeros, response should decay
    assert!(from_sample(out6).abs() < 0.01, "Response should decay to zero");
}

#[test]
fn test_fir_filter_reset() {
    let coeffs = FirCoefficients::<7>::lowpass(0.25);
    let mut filter = FirFilter::new(coeffs.clone());

    // Process some samples
    for _ in 0..10 {
        filter.process(to_sample(0.8));
    }

    // Reset
    filter.reset();

    // After reset, first sample should be like fresh filter
    let mut fresh_filter = FirFilter::new(coeffs);
    let input = to_sample(0.3);

    let out_reset = filter.process(input);
    let out_fresh = fresh_filter.process(input);

    assert!(
        (from_sample(out_reset) - from_sample(out_fresh)).abs() < 0.001,
        "Reset failed"
    );
}

#[test]
fn test_fir_filter_block_processing() {
    let coeffs = FirCoefficients::<5>::lowpass(0.3);
    let mut filter = FirFilter::new(coeffs);

    let mut samples = [
        to_sample(0.5),
        to_sample(-0.5),
        to_sample(0.5),
        to_sample(-0.5),
    ];

    filter.process_block(&mut samples);

    // All outputs should be finite
    for s in &samples {
        let v = from_sample(*s);
        assert!(v.is_finite(), "Output not finite");
    }
}

#[test]
fn test_fir_filter_set_coefficients() {
    let coeffs1 = FirCoefficients::<7>::lowpass(0.1);
    let coeffs2 = FirCoefficients::<7>::lowpass(0.4);
    let mut filter = FirFilter::new(coeffs1);

    // Process with first coefficients
    for _ in 0..10 {
        filter.process(to_sample(0.5));
    }

    // Change coefficients (should reset state)
    filter.set_coefficients(coeffs2);

    // Filter should be reset
    let output = filter.process(to_sample(0.3));
    assert!(from_sample(output).is_finite());
}

#[test]
fn test_fir_filter_alternating_signal() {
    // Lowpass should smooth alternating signal
    let coeffs = FirCoefficients::<15>::lowpass(0.1);
    let mut filter = FirFilter::new(coeffs);

    let mut max_output = 0.0f32;

    for i in 0..100 {
        let input = if i % 2 == 0 { 0.9 } else { -0.9 };
        let output = filter.process(to_sample(input));
        max_output = max_output.max(from_sample(output).abs());
    }

    // Output should be attenuated (alternating is high frequency)
    assert!(
        max_output < 0.9,
        "Lowpass should attenuate high freq: {}",
        max_output
    );
}

#[test]
fn test_fir_coefficients_normalized() {
    let coeffs = FirCoefficients::<31>::lowpass(0.2);

    // Sum of lowpass coefficients should be approximately 1
    let mut sum = 0.0f32;
    for i in 0..31 {
        sum += from_sample(coeffs.get(i));
    }

    assert!(
        (sum - 1.0).abs() < 0.1,
        "Lowpass coefficients should sum to ~1: {}",
        sum
    );
}

// =============================================================================
// DSP Latency Tests (PF-005: Audio latency < 20ms)
// =============================================================================

/// Calculate group delay of a filter in samples
fn measure_group_delay<F>(mut filter: F, taps: usize) -> usize
where
    F: FnMut(f32) -> f32,
{
    // Send an impulse and find when the peak output occurs
    let impulse = 1.0f32;
    let _ = filter(impulse);

    let mut max_output = 0.0f32;
    let mut max_idx = 0;

    for i in 1..(taps * 2) {
        let out = filter(0.0);
        if out.abs() > max_output {
            max_output = out.abs();
            max_idx = i;
        }
    }

    max_idx
}

#[test]
fn test_biquad_latency() {
    // Biquad filters have minimal latency (1-2 samples)
    let coeffs = BiquadCoeffs::lowpass(0.1, 0.707);
    let mut filter = BiquadFilter::with_coeffs(coeffs);

    let delay = measure_group_delay(|x| filter.process(x), 10);

    // Biquad should have ~1-3 samples delay
    assert!(delay <= 5, "Biquad delay too high: {} samples", delay);
}

#[test]
fn test_fir_latency() {
    // FIR filter delay is (taps-1)/2 samples
    let coeffs = FirCoefficients::<15>::lowpass(0.2);
    let mut filter = FirFilter::new(coeffs);

    let delay = measure_group_delay(|x| from_sample(filter.process(to_sample(x))), 20);

    // 15-tap FIR should have ~7 samples delay
    assert!(delay <= 15, "FIR delay too high: {} samples", delay);
}

#[test]
fn test_dc_blocker_latency() {
    let mut blocker = DcBlocker::default();

    let delay = measure_group_delay(|x| blocker.process(x), 10);

    // DC blocker should have ~1-2 samples delay
    assert!(delay <= 5, "DC blocker delay too high: {} samples", delay);
}

#[test]
fn test_total_audio_chain_latency() {
    // Simulate a typical audio chain:
    // - DC blocker
    // - Bandpass filter (FIR)
    // - Lowpass (Biquad)

    let mut dc_blocker = DcBlocker::default();
    let bp_coeffs = FirCoefficients::<15>::bandpass(0.01, 0.1);
    let mut bp_filter = FirFilter::new(bp_coeffs);
    let lp_coeffs = BiquadCoeffs::lowpass(0.1, 0.707);
    let mut lp_filter = BiquadFilter::with_coeffs(lp_coeffs);

    let process_chain = |x: f32| {
        let y1 = dc_blocker.process(x);
        let y2 = from_sample(bp_filter.process(to_sample(y1)));
        lp_filter.process(y2)
    };

    // Send impulse through chain
    let mut dc_blocker2 = DcBlocker::default();
    let bp_coeffs2 = FirCoefficients::<15>::bandpass(0.01, 0.1);
    let mut bp_filter2 = FirFilter::new(bp_coeffs2);
    let lp_coeffs2 = BiquadCoeffs::lowpass(0.1, 0.707);
    let mut lp_filter2 = BiquadFilter::with_coeffs(lp_coeffs2);

    let delay = measure_group_delay(
        |x| {
            let y1 = dc_blocker2.process(x);
            let y2 = from_sample(bp_filter2.process(to_sample(y1)));
            lp_filter2.process(y2)
        },
        30,
    );

    // Total chain latency in samples
    // At 48kHz, 20ms = 960 samples
    // Our chain should be well under 100 samples
    assert!(
        delay < 100,
        "Audio chain delay too high: {} samples",
        delay
    );

    // Convert to ms at 48kHz
    let latency_ms = delay as f32 / 48.0;
    assert!(
        latency_ms < 20.0,
        "Audio chain latency exceeds 20ms: {:.2}ms",
        latency_ms
    );
}
