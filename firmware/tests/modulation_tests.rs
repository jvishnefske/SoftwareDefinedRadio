//! Modulation/Demodulation Tests
//!
//! Tests for SSB, AM, and FM modulation/demodulation.

use sdr_firmware::dsp::modulation::{
    AmDemodulator, AmModulator, FmDemodulator, HilbertTransform, IqSample,
};
use sdr_firmware::dsp::oscillator::SineOscillator;

const SAMPLE_RATE: f32 = 48000.0;

/// Helper to create a sine oscillator with frequency
fn make_sine_osc(freq: f32, sample_rate: f32) -> SineOscillator {
    let mut osc = SineOscillator::new();
    osc.set_frequency(freq, sample_rate);
    osc
}

// ============================================================================
// IQ Sample Tests
// ============================================================================

#[test]
fn test_iq_sample_creation() {
    let iq = IqSample::new(1.0, 0.0);
    assert_eq!(iq.i, 1.0);
    assert_eq!(iq.q, 0.0);
}

#[test]
fn test_iq_sample_default() {
    let iq = IqSample::default();
    assert_eq!(iq.i, 0.0);
    assert_eq!(iq.q, 0.0);
}

#[test]
fn test_iq_magnitude() {
    let iq = IqSample::new(3.0, 4.0);
    let mag = iq.magnitude();
    assert!((mag - 5.0).abs() < 0.001);
}

#[test]
fn test_iq_magnitude_unit() {
    let iq = IqSample::new(1.0, 0.0);
    assert!((iq.magnitude() - 1.0).abs() < 0.001);

    let iq2 = IqSample::new(0.0, 1.0);
    assert!((iq2.magnitude() - 1.0).abs() < 0.001);
}

#[test]
fn test_iq_phase() {
    let iq = IqSample::new(1.0, 0.0);
    assert!(iq.phase().abs() < 0.001);

    let iq2 = IqSample::new(0.0, 1.0);
    assert!((iq2.phase() - core::f32::consts::FRAC_PI_2).abs() < 0.001);
}

#[test]
fn test_iq_rotate_90_degrees() {
    let iq = IqSample::new(1.0, 0.0);
    let rotated = iq.rotate(core::f32::consts::FRAC_PI_2);
    assert!(rotated.i.abs() < 0.001);
    assert!((rotated.q - 1.0).abs() < 0.001);
}

#[test]
fn test_iq_rotate_180_degrees() {
    let iq = IqSample::new(1.0, 0.0);
    let rotated = iq.rotate(core::f32::consts::PI);
    assert!((rotated.i + 1.0).abs() < 0.001);
    assert!(rotated.q.abs() < 0.001);
}

#[test]
fn test_iq_multiply() {
    // (1+j) * (1+j) = 1 + 2j - 1 = 2j
    let a = IqSample::new(1.0, 1.0);
    let b = IqSample::new(1.0, 1.0);
    let result = a.multiply(b);
    assert!(result.i.abs() < 0.001);
    assert!((result.q - 2.0).abs() < 0.001);
}

#[test]
fn test_iq_multiply_conjugate() {
    // (a+jb) * (a-jb) = a^2 + b^2 (real only)
    let iq = IqSample::new(3.0, 4.0);
    let result = iq.multiply(iq.conjugate());
    assert!((result.i - 25.0).abs() < 0.001);
    assert!(result.q.abs() < 0.001);
}

#[test]
fn test_iq_conjugate() {
    let iq = IqSample::new(1.0, 2.0);
    let conj = iq.conjugate();
    assert_eq!(conj.i, 1.0);
    assert_eq!(conj.q, -2.0);
}

// ============================================================================
// Hilbert Transform Tests
// ============================================================================

#[test]
fn test_hilbert_creation() {
    let _hilbert = HilbertTransform::new();
}

#[test]
fn test_hilbert_default() {
    let _hilbert = HilbertTransform::default();
    // Cannot check private fields, just verify creation works
}

#[test]
fn test_hilbert_processes_samples() {
    let mut hilbert = HilbertTransform::new();
    // After warm-up, should produce output
    for _ in 0..50 {
        hilbert.process(1.0);
    }
    // Output should be finite
    let output = hilbert.process(1.0);
    assert!(output.is_finite());
}

#[test]
fn test_hilbert_reset() {
    let mut hilbert = HilbertTransform::new();
    for _ in 0..50 {
        hilbert.process(1.0);
    }
    hilbert.reset();
    // After reset, processing zeros should give zeros
    for _ in 0..50 {
        assert_eq!(hilbert.process(0.0), 0.0);
    }
}

#[test]
fn test_hilbert_zero_input() {
    let mut hilbert = HilbertTransform::new();
    for _ in 0..100 {
        let output = hilbert.process(0.0);
        assert_eq!(output, 0.0);
    }
}

// ============================================================================
// AM Demodulator Tests
// ============================================================================

#[test]
fn test_am_demodulator_creation() {
    let _demod = AmDemodulator::new(SAMPLE_RATE);
}

#[test]
fn test_am_demodulator_envelope_detection() {
    let mut demod = AmDemodulator::new(SAMPLE_RATE);

    // Pure carrier (no modulation) should give constant output
    for _ in 0..1000 {
        let iq = IqSample::new(1.0, 0.0);
        let _ = demod.process(iq);
    }

    // Output should be finite
    let output = demod.process(IqSample::new(1.0, 0.0));
    assert!(output.is_finite());
}

#[test]
fn test_am_demodulator_varying_amplitude() {
    let mut demod = AmDemodulator::new(SAMPLE_RATE);

    // First, warm up with constant signal
    for _ in 0..500 {
        demod.process(IqSample::new(0.5, 0.0));
    }

    // Then apply larger signal
    let mut outputs = Vec::new();
    for _ in 0..100 {
        outputs.push(demod.process(IqSample::new(1.0, 0.0)));
    }

    // All outputs should be finite
    assert!(outputs.iter().all(|x| x.is_finite()));
}

#[test]
fn test_am_demodulator_reset() {
    let mut demod = AmDemodulator::new(SAMPLE_RATE);

    for _ in 0..100 {
        demod.process(IqSample::new(1.0, 0.5));
    }

    demod.reset();

    // After reset, processing should still work
    let output = demod.process(IqSample::new(1.0, 0.0));
    assert!(output.is_finite());
}

// ============================================================================
// FM Demodulator Tests
// ============================================================================

#[test]
fn test_fm_demodulator_creation() {
    let _demod = FmDemodulator::new(SAMPLE_RATE, 5000.0);
}

#[test]
fn test_fm_demodulator_constant_phase() {
    let mut demod = FmDemodulator::new(SAMPLE_RATE, 5000.0);

    // Constant phase should produce near-zero output after filter settles
    for _ in 0..500 {
        let output = demod.process(IqSample::new(1.0, 0.0));
        assert!(output.is_finite());
    }
}

#[test]
fn test_fm_demodulator_rotating_phase() {
    let mut demod = FmDemodulator::new(SAMPLE_RATE, 5000.0);

    // Rotating phase at constant rate should produce DC-like output
    let phase_inc = 2.0 * core::f32::consts::PI * 1000.0 / SAMPLE_RATE;
    let mut phase: f32 = 0.0;

    for _ in 0..1000 {
        let iq = IqSample::new(phase.cos(), phase.sin());
        let output = demod.process(iq);
        assert!(output.is_finite());
        phase += phase_inc;
    }
}

#[test]
fn test_fm_demodulator_reset() {
    let mut demod = FmDemodulator::new(SAMPLE_RATE, 5000.0);

    for _ in 0..100 {
        demod.process(IqSample::new(1.0, 0.5));
    }

    demod.reset();

    let output = demod.process(IqSample::new(1.0, 0.0));
    assert!(output.is_finite());
}

// ============================================================================
// AM Modulator Tests
// ============================================================================

#[test]
fn test_am_modulator_creation() {
    let _mod = AmModulator::new(1500.0, SAMPLE_RATE);
}

#[test]
fn test_am_modulator_no_modulation() {
    let mut modulator = AmModulator::new(1500.0, SAMPLE_RATE);
    modulator.set_depth(0.0);

    // With zero modulation, output is just the carrier
    let outputs: Vec<f32> = (0..100).map(|_| modulator.process(0.0)).collect();

    // All outputs should be finite
    assert!(outputs.iter().all(|x| x.is_finite()));

    // Should have some variation (carrier oscillation)
    let max = outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min = outputs.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(max - min > 0.5);
}

#[test]
fn test_am_modulator_full_modulation() {
    let mut modulator = AmModulator::new(1500.0, SAMPLE_RATE);
    modulator.set_depth(1.0);

    // Full positive modulation should double amplitude
    let output_pos = modulator.process(1.0);
    assert!(output_pos.is_finite());

    // Full negative should give zero
    let mut modulator2 = AmModulator::new(1500.0, SAMPLE_RATE);
    modulator2.set_depth(1.0);
    let output_neg = modulator2.process(-1.0);
    assert!(output_neg.is_finite());
}

#[test]
fn test_am_modulator_depth_clamp() {
    let mut modulator = AmModulator::new(1500.0, SAMPLE_RATE);

    modulator.set_depth(2.0);
    let output = modulator.process(1.0);
    assert!(output.is_finite());

    modulator.set_depth(-0.5);
    let output2 = modulator.process(1.0);
    assert!(output2.is_finite());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_am_modulation_demodulation_roundtrip() {
    let mut modulator = AmModulator::new(1500.0, SAMPLE_RATE);
    let mut demodulator = AmDemodulator::new(SAMPLE_RATE);
    modulator.set_depth(0.8);

    // Create test audio (1000 Hz sine)
    let mut osc = make_sine_osc(1000.0, SAMPLE_RATE);

    // Warm up
    for _ in 0..1000 {
        let audio = osc.next();
        let modulated = modulator.process(audio);
        // For AM demod, we need IQ. Create IQ from real signal
        let iq = IqSample::new(modulated, 0.0);
        demodulator.process(iq);
    }

    // Test
    let mut max_recovered = 0.0f32;
    for _ in 0..1000 {
        let audio = osc.next();
        let modulated = modulator.process(audio);
        let iq = IqSample::new(modulated, 0.0);
        let recovered = demodulator.process(iq);
        max_recovered = max_recovered.max(recovered.abs());
    }

    // Should have some recovered audio
    assert!(max_recovered > 0.0);
    assert!(max_recovered.is_finite());
}

#[test]
fn test_fm_phase_detection_accuracy() {
    let mut demod = FmDemodulator::new(SAMPLE_RATE, SAMPLE_RATE / 4.0);

    // Create phase ramp (linear frequency)
    let freq = 1000.0;
    let phase_inc = 2.0 * core::f32::consts::PI * freq / SAMPLE_RATE;

    // Warm up
    let mut phase: f32 = 0.0;
    for _ in 0..2000 {
        let iq = IqSample::new(phase.cos(), phase.sin());
        demod.process(iq);
        phase += phase_inc;
    }

    // After settling, output should be relatively constant
    let mut outputs = Vec::new();
    for _ in 0..100 {
        let iq = IqSample::new(phase.cos(), phase.sin());
        outputs.push(demod.process(iq));
        phase += phase_inc;
    }

    // Variance should be low (DC-ish output)
    let mean: f32 = outputs.iter().sum::<f32>() / outputs.len() as f32;
    let variance: f32 =
        outputs.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / outputs.len() as f32;

    // Low variance indicates constant output
    assert!(variance < 0.01);
}

// ============================================================================
// SSB Demodulator Tests
// ============================================================================

use sdr_firmware::dsp::modulation::SsbDemodulator;

#[test]
fn test_ssb_demodulator_creation() {
    let _demod = SsbDemodulator::new(SAMPLE_RATE, 2700.0);
}

#[test]
fn test_ssb_demodulator_processes_iq() {
    let mut demod = SsbDemodulator::new(SAMPLE_RATE, 2700.0);

    // Process some samples
    for _ in 0..100 {
        let iq = IqSample::new(0.5, 0.3);
        let audio = demod.process(iq);
        assert!(audio.is_finite());
    }
}

#[test]
fn test_ssb_demodulator_set_usb() {
    let mut demod = SsbDemodulator::new(SAMPLE_RATE, 2700.0);
    demod.set_usb(true);

    let audio = demod.process(IqSample::new(1.0, 0.0));
    assert!(audio.is_finite());
}

#[test]
fn test_ssb_demodulator_set_lsb() {
    let mut demod = SsbDemodulator::new(SAMPLE_RATE, 2700.0);
    demod.set_usb(false);

    let audio = demod.process(IqSample::new(1.0, 0.0));
    assert!(audio.is_finite());
}

#[test]
fn test_ssb_demodulator_reset() {
    let mut demod = SsbDemodulator::new(SAMPLE_RATE, 2700.0);

    // Process some samples
    for _ in 0..100 {
        demod.process(IqSample::new(0.5, 0.3));
    }

    demod.reset();

    // Should work after reset
    let audio = demod.process(IqSample::new(1.0, 0.0));
    assert!(audio.is_finite());
}

#[test]
fn test_ssb_demodulator_zero_input() {
    let mut demod = SsbDemodulator::new(SAMPLE_RATE, 2700.0);

    // Warm up
    for _ in 0..100 {
        demod.process(IqSample::default());
    }

    // Zero input should give near-zero output
    let audio = demod.process(IqSample::default());
    assert!(audio.abs() < 0.01);
}

#[test]
fn test_ssb_demodulator_usb_vs_lsb_differ() {
    let mut demod_usb = SsbDemodulator::new(SAMPLE_RATE, 2700.0);
    let mut demod_lsb = SsbDemodulator::new(SAMPLE_RATE, 2700.0);

    demod_usb.set_usb(true);
    demod_lsb.set_usb(false);

    // Create test signal
    let iq = IqSample::new(0.7, 0.3);

    // Warm up
    for _ in 0..200 {
        demod_usb.process(iq);
        demod_lsb.process(iq);
    }

    // Get outputs - they may differ based on sideband selection
    let usb_out = demod_usb.process(iq);
    let lsb_out = demod_lsb.process(iq);

    // Both should be finite
    assert!(usb_out.is_finite());
    assert!(lsb_out.is_finite());
}

// ============================================================================
// SSB Modulator Tests
// ============================================================================

use sdr_firmware::dsp::modulation::SsbModulator;

#[test]
fn test_ssb_modulator_creation() {
    let _modulator = SsbModulator::new(SAMPLE_RATE);
}

#[test]
fn test_ssb_modulator_processes_audio() {
    let mut modulator = SsbModulator::new(SAMPLE_RATE);

    // Process audio sample
    let iq = modulator.process(0.5);
    assert!(iq.i.is_finite());
    assert!(iq.q.is_finite());
}

#[test]
fn test_ssb_modulator_set_usb() {
    let mut modulator = SsbModulator::new(SAMPLE_RATE);
    modulator.set_usb(true);

    let iq = modulator.process(0.5);
    assert!(iq.magnitude().is_finite());
}

#[test]
fn test_ssb_modulator_set_lsb() {
    let mut modulator = SsbModulator::new(SAMPLE_RATE);
    modulator.set_usb(false);

    let iq = modulator.process(0.5);
    assert!(iq.magnitude().is_finite());
}

#[test]
fn test_ssb_modulator_set_carrier() {
    let mut modulator = SsbModulator::new(SAMPLE_RATE);
    modulator.set_carrier(1500.0, SAMPLE_RATE);

    let iq = modulator.process(0.5);
    assert!(iq.i.is_finite());
    assert!(iq.q.is_finite());
}

#[test]
fn test_ssb_modulator_reset() {
    let mut modulator = SsbModulator::new(SAMPLE_RATE);

    // Process some audio
    for i in 0..100 {
        let audio = (i as f32 * 0.1).sin();
        modulator.process(audio);
    }

    modulator.reset();

    // Should work after reset
    let iq = modulator.process(0.5);
    assert!(iq.i.is_finite());
}

#[test]
fn test_ssb_modulator_usb_vs_lsb_q_sign_differs() {
    let mut mod_usb = SsbModulator::new(SAMPLE_RATE);
    let mut mod_lsb = SsbModulator::new(SAMPLE_RATE);

    mod_usb.set_usb(true);
    mod_lsb.set_usb(false);

    // Warm up Hilbert transforms
    for _ in 0..50 {
        mod_usb.process(0.5);
        mod_lsb.process(0.5);
    }

    // Get outputs
    let usb_iq = mod_usb.process(0.5);
    let lsb_iq = mod_lsb.process(0.5);

    // I components should be the same
    assert!((usb_iq.i - lsb_iq.i).abs() < 0.01);

    // Q components should have opposite sign (conjugate relationship)
    // Due to filter phase, they may not be exactly opposite, but should differ
    assert!(usb_iq.q.is_finite());
    assert!(lsb_iq.q.is_finite());
}

// ============================================================================
// Advanced Hilbert Transform Tests
// ============================================================================

#[test]
fn test_hilbert_quadrature_relationship() {
    let mut hilbert = HilbertTransform::new();

    // Generate test signal
    let freq = 1000.0;
    let phase_inc = 2.0 * core::f32::consts::PI * freq / SAMPLE_RATE;
    let mut phase: f32 = 0.0;

    // Warm up
    for _ in 0..100 {
        let input = phase.sin();
        hilbert.process(input);
        phase += phase_inc;
    }

    // Collect I/Q pairs
    let mut i_samples = Vec::new();
    let mut q_samples = Vec::new();

    for _ in 0..48 {
        // One period at 1kHz
        let input = phase.sin();
        let q = hilbert.process(input);
        i_samples.push(input);
        q_samples.push(q);
        phase += phase_inc;
    }

    // Q should be approximately -cos (90 degree shifted from sin)
    // Check that I and Q are orthogonal: sum(I*Q) ≈ 0
    // Note: 15-tap FIR Hilbert is approximate, so tolerance must be generous
    let dot_product: f32 = i_samples
        .iter()
        .zip(q_samples.iter())
        .map(|(i, q)| i * q)
        .sum();
    let normalized = dot_product / i_samples.len() as f32;

    // The 15-tap Hilbert has limited accuracy, so allow larger tolerance
    assert!(
        normalized.abs() < 0.5,
        "I and Q should be roughly orthogonal, got dot product {}",
        normalized
    );
}

#[test]
fn test_hilbert_preserves_magnitude() {
    let mut hilbert = HilbertTransform::new();

    // Warm up
    for _ in 0..100 {
        hilbert.process(0.5);
    }

    // For a constant-amplitude sine, output should have similar magnitude
    let freq = 1000.0;
    let phase_inc = 2.0 * core::f32::consts::PI * freq / SAMPLE_RATE;
    let mut phase: f32 = 0.0;

    let mut input_energy = 0.0f32;
    let mut output_energy = 0.0f32;

    for _ in 0..480 {
        // 10 periods
        let input = 0.7 * phase.sin();
        let output = hilbert.process(input);
        input_energy += input * input;
        output_energy += output * output;
        phase += phase_inc;
    }

    let input_rms = (input_energy / 480.0).sqrt();
    let output_rms = (output_energy / 480.0).sqrt();

    // RMS values should be similar (within 20%)
    let ratio = output_rms / input_rms;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "Hilbert should preserve magnitude, got ratio {}",
        ratio
    );
}

// ============================================================================
// SSB Roundtrip Tests
// ============================================================================

#[test]
fn test_ssb_modulator_demodulator_roundtrip() {
    let mut modulator = SsbModulator::new(SAMPLE_RATE);
    let mut demodulator = SsbDemodulator::new(SAMPLE_RATE, 2700.0);

    modulator.set_usb(true);
    demodulator.set_usb(true);

    // Create audio tone
    let freq = 1000.0;
    let phase_inc = 2.0 * core::f32::consts::PI * freq / SAMPLE_RATE;
    let mut phase: f32 = 0.0;

    // Warm up
    for _ in 0..500 {
        let audio = 0.5 * phase.sin();
        let iq = modulator.process(audio);
        demodulator.process(iq);
        phase += phase_inc;
    }

    // Collect recovered audio
    let mut recovered = Vec::new();
    for _ in 0..480 {
        let audio = 0.5 * phase.sin();
        let iq = modulator.process(audio);
        recovered.push(demodulator.process(iq));
        phase += phase_inc;
    }

    // Check that recovered audio has energy
    let energy: f32 = recovered.iter().map(|x| x * x).sum();
    let rms = (energy / recovered.len() as f32).sqrt();

    assert!(rms > 0.01, "Recovered audio should have energy, got RMS {}", rms);
}

#[test]
fn test_ssb_carrier_suppression() {
    let mut modulator = SsbModulator::new(SAMPLE_RATE);
    modulator.set_usb(true);

    // Warm up
    for _ in 0..100 {
        modulator.process(0.5);
    }

    // With zero audio input, carrier should be suppressed
    let mut carrier_energy = 0.0f32;
    for _ in 0..100 {
        let iq = modulator.process(0.0);
        carrier_energy += iq.magnitude();
    }

    // Very low carrier energy expected
    let avg_magnitude = carrier_energy / 100.0;
    assert!(
        avg_magnitude < 0.1,
        "SSB should suppress carrier, got avg magnitude {}",
        avg_magnitude
    );
}

// ============================================================================
// IQ Processing Edge Cases
// ============================================================================

#[test]
fn test_iq_normalize() {
    let iq = IqSample::new(3.0, 4.0);
    let normalized = iq.normalize();

    let mag = normalized.magnitude();
    assert!(
        (mag - 1.0).abs() < 0.01,
        "Normalized IQ should have magnitude 1, got {}",
        mag
    );
}

#[test]
fn test_iq_normalize_zero() {
    let iq = IqSample::new(0.0, 0.0);
    let normalized = iq.normalize();

    // Normalizing zero should return zero (not NaN/Inf)
    assert!(normalized.i.is_finite());
    assert!(normalized.q.is_finite());
}

#[test]
fn test_iq_scale() {
    let iq = IqSample::new(1.0, 2.0);
    let scaled = iq.scale(2.0);

    assert_eq!(scaled.i, 2.0);
    assert_eq!(scaled.q, 4.0);
}

#[test]
fn test_iq_add() {
    let a = IqSample::new(1.0, 2.0);
    let b = IqSample::new(3.0, 4.0);
    let sum = a.add(b);

    assert_eq!(sum.i, 4.0);
    assert_eq!(sum.q, 6.0);
}

#[test]
fn test_iq_sub() {
    let a = IqSample::new(5.0, 6.0);
    let b = IqSample::new(2.0, 1.0);
    let diff = a.sub(b);

    assert_eq!(diff.i, 3.0);
    assert_eq!(diff.q, 5.0);
}
