//! Encoder Logic Tests
//!
//! Tests for quadrature decoder and acceleration curve.
//! Run with: cargo test --target x86_64-unknown-linux-gnu --no-default-features --features std --test encoder_tests

// The encoder module is gated behind embedded feature, so we test the logic inline

/// Direction enum for testing
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Clockwise,
    CounterClockwise,
}

/// Encoder state machine states
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum EncoderState {
    #[default]
    Idle,
    CwStart,
    CwNext,
    CwFinal,
    CcwStart,
    CcwNext,
    CcwFinal,
}

/// Quadrature decoder (copy of implementation for testing)
struct QuadratureDecoder {
    state: EncoderState,
    last_a: bool,
    last_b: bool,
}

impl QuadratureDecoder {
    fn new() -> Self {
        Self {
            state: EncoderState::Idle,
            last_a: false,
            last_b: false,
        }
    }

    fn update(&mut self, a: bool, b: bool) -> Option<Direction> {
        use EncoderState::*;

        if a == self.last_a && b == self.last_b {
            return None;
        }

        self.last_a = a;
        self.last_b = b;

        let (new_state, result) = match (self.state, a, b) {
            (Idle, false, true) => (CwStart, None),
            (Idle, true, false) => (CcwStart, None),
            (CwStart, true, true) => (CwNext, None),
            (CwNext, true, false) => (CwFinal, None),
            (CwFinal, false, false) => (Idle, Some(Direction::Clockwise)),
            (CcwStart, true, true) => (CcwNext, None),
            (CcwNext, false, true) => (CcwFinal, None),
            (CcwFinal, false, false) => (Idle, Some(Direction::CounterClockwise)),
            _ => (Idle, None),
        };

        self.state = new_state;
        result
    }

    fn reset(&mut self) {
        self.state = EncoderState::Idle;
    }
}

/// Acceleration curve (copy of implementation for testing)
struct AccelerationCurve {
    threshold_ms: u32,
    multiplier: u32,
    last_event_ms: u32,
    step_count: u32,
}

impl AccelerationCurve {
    fn new(threshold_ms: u32, multiplier: u32) -> Self {
        Self {
            threshold_ms,
            multiplier,
            last_event_ms: 0,
            step_count: 0,
        }
    }

    fn process(&mut self, current_ms: u32) -> u32 {
        let elapsed = current_ms.wrapping_sub(self.last_event_ms);
        self.last_event_ms = current_ms;

        if elapsed < self.threshold_ms {
            self.step_count = self.step_count.saturating_add(1).min(10);
            1 + (self.step_count * self.multiplier / 10)
        } else {
            self.step_count = 0;
            1
        }
    }

    fn reset(&mut self) {
        self.step_count = 0;
    }
}

/// Bounded value (copy of implementation for testing)
#[derive(Clone, Copy, Debug)]
struct BoundedValue<T> {
    value: T,
    min: T,
    max: T,
}

impl<T: Copy + Ord> BoundedValue<T> {
    fn new(value: T, min: T, max: T) -> Self {
        Self { value, min, max }
    }

    fn get(&self) -> T {
        self.value
    }

    fn set(&mut self, value: T) {
        self.value = if value < self.min {
            self.min
        } else if value > self.max {
            self.max
        } else {
            value
        };
    }
}

impl BoundedValue<i32> {
    fn increment(&mut self, amount: i32) {
        self.set(self.value.saturating_add(amount));
    }

    fn decrement(&mut self, amount: i32) {
        self.set(self.value.saturating_sub(amount));
    }

    fn handle_rotation(&mut self, direction: Direction, steps: u32) {
        let delta = steps as i32;
        match direction {
            Direction::Clockwise => self.increment(delta),
            Direction::CounterClockwise => self.decrement(delta),
        }
    }
}

// =============================================================================
// Quadrature Decoder Tests
// =============================================================================

#[test]
fn decoder_creation() {
    let decoder = QuadratureDecoder::new();
    assert_eq!(decoder.state, EncoderState::Idle);
}

#[test]
fn decoder_no_change_no_event() {
    let mut decoder = QuadratureDecoder::new();
    // Same state as initial (false, false)
    assert!(decoder.update(false, false).is_none());
}

#[test]
fn decoder_clockwise_full_cycle() {
    let mut decoder = QuadratureDecoder::new();

    // CW sequence: 00 -> 01 -> 11 -> 10 -> 00
    assert!(decoder.update(false, true).is_none()); // Start CW
    assert!(decoder.update(true, true).is_none());  // Next
    assert!(decoder.update(true, false).is_none()); // Final
    assert_eq!(decoder.update(false, false), Some(Direction::Clockwise)); // Complete
}

#[test]
fn decoder_counter_clockwise_full_cycle() {
    let mut decoder = QuadratureDecoder::new();

    // CCW sequence: 00 -> 10 -> 11 -> 01 -> 00
    assert!(decoder.update(true, false).is_none()); // Start CCW
    assert!(decoder.update(true, true).is_none());  // Next
    assert!(decoder.update(false, true).is_none()); // Final
    assert_eq!(decoder.update(false, false), Some(Direction::CounterClockwise)); // Complete
}

#[test]
fn decoder_partial_cw_no_event() {
    let mut decoder = QuadratureDecoder::new();

    // Partial CW, then back to idle
    assert!(decoder.update(false, true).is_none());
    assert!(decoder.update(true, true).is_none());
    // Invalid transition resets to idle
    decoder.reset();
    assert_eq!(decoder.state, EncoderState::Idle);
}

#[test]
fn decoder_multiple_cw_steps() {
    let mut decoder = QuadratureDecoder::new();

    // First CW step
    decoder.update(false, true);
    decoder.update(true, true);
    decoder.update(true, false);
    assert_eq!(decoder.update(false, false), Some(Direction::Clockwise));

    // Second CW step
    decoder.update(false, true);
    decoder.update(true, true);
    decoder.update(true, false);
    assert_eq!(decoder.update(false, false), Some(Direction::Clockwise));
}

#[test]
fn decoder_direction_change() {
    let mut decoder = QuadratureDecoder::new();

    // Start CW
    decoder.update(false, true);
    decoder.update(true, true);
    decoder.update(true, false);
    assert_eq!(decoder.update(false, false), Some(Direction::Clockwise));

    // Now CCW
    decoder.update(true, false);
    decoder.update(true, true);
    decoder.update(false, true);
    assert_eq!(decoder.update(false, false), Some(Direction::CounterClockwise));
}

#[test]
fn decoder_reset() {
    let mut decoder = QuadratureDecoder::new();

    decoder.update(false, true);
    decoder.update(true, true);
    decoder.reset();

    assert_eq!(decoder.state, EncoderState::Idle);
}

// =============================================================================
// Acceleration Curve Tests
// =============================================================================

#[test]
fn acceleration_creation() {
    let accel = AccelerationCurve::new(50, 5);
    assert_eq!(accel.threshold_ms, 50);
    assert_eq!(accel.multiplier, 5);
}

#[test]
fn acceleration_slow_rotation() {
    let mut accel = AccelerationCurve::new(50, 5);

    // Slow rotation (100ms apart) should give 1 step
    assert_eq!(accel.process(0), 1);
    assert_eq!(accel.process(100), 1);
    assert_eq!(accel.process(200), 1);
}

#[test]
fn acceleration_fast_rotation() {
    let mut accel = AccelerationCurve::new(50, 5);

    // Fast rotation (20ms apart) should accelerate
    accel.process(0);
    let step1 = accel.process(20);
    let step2 = accel.process(40);
    let step3 = accel.process(60);

    assert!(step2 >= step1, "Should accelerate");
    assert!(step3 >= step2, "Should continue accelerating");
}

#[test]
fn acceleration_max_limit() {
    let mut accel = AccelerationCurve::new(50, 5);

    // Very fast rotation should cap at max
    accel.process(0);
    for i in 1..=20 {
        let steps = accel.process(i * 10);
        // With multiplier 5 and max step_count 10:
        // max = 1 + (10 * 5 / 10) = 1 + 5 = 6
        assert!(steps <= 6, "Steps should be capped at {}, got {}", 6, steps);
    }
}

#[test]
fn acceleration_reset_after_slow() {
    let mut accel = AccelerationCurve::new(50, 5);

    // Fast rotation
    accel.process(0);
    accel.process(20);
    accel.process(40);
    let fast_steps = accel.process(60);

    // Long pause
    let after_pause = accel.process(200);

    assert_eq!(after_pause, 1, "Should reset after slow rotation");
    assert!(fast_steps > after_pause, "Fast should be more than slow");
}

#[test]
fn acceleration_reset_explicit() {
    let mut accel = AccelerationCurve::new(50, 5);

    accel.process(0);
    accel.process(20);
    accel.process(40);

    accel.reset();
    assert_eq!(accel.step_count, 0);
}

// =============================================================================
// Bounded Value Tests
// =============================================================================

#[test]
fn bounded_value_creation() {
    let val = BoundedValue::new(50, 0, 100);
    assert_eq!(val.get(), 50);
}

#[test]
fn bounded_value_set_within_range() {
    let mut val = BoundedValue::new(50, 0, 100);
    val.set(75);
    assert_eq!(val.get(), 75);
}

#[test]
fn bounded_value_set_clamp_high() {
    let mut val = BoundedValue::new(50, 0, 100);
    val.set(150);
    assert_eq!(val.get(), 100);
}

#[test]
fn bounded_value_set_clamp_low() {
    let mut val: BoundedValue<i32> = BoundedValue::new(50, 0, 100);
    val.set(-50);
    assert_eq!(val.get(), 0);
}

#[test]
fn bounded_value_increment() {
    let mut val = BoundedValue::new(50, 0, 100);
    val.increment(10);
    assert_eq!(val.get(), 60);
}

#[test]
fn bounded_value_increment_clamp() {
    let mut val = BoundedValue::new(95, 0, 100);
    val.increment(10);
    assert_eq!(val.get(), 100);
}

#[test]
fn bounded_value_decrement() {
    let mut val = BoundedValue::new(50, 0, 100);
    val.decrement(10);
    assert_eq!(val.get(), 40);
}

#[test]
fn bounded_value_decrement_clamp() {
    let mut val = BoundedValue::new(5, 0, 100);
    val.decrement(10);
    assert_eq!(val.get(), 0);
}

#[test]
fn bounded_value_handle_rotation_cw() {
    let mut val = BoundedValue::new(50, 0, 100);
    val.handle_rotation(Direction::Clockwise, 5);
    assert_eq!(val.get(), 55);
}

#[test]
fn bounded_value_handle_rotation_ccw() {
    let mut val = BoundedValue::new(50, 0, 100);
    val.handle_rotation(Direction::CounterClockwise, 5);
    assert_eq!(val.get(), 45);
}

#[test]
fn bounded_value_negative_range() {
    let mut val = BoundedValue::new(0, -100, 100);
    val.decrement(50);
    assert_eq!(val.get(), -50);
}

#[test]
fn bounded_value_at_min() {
    let val = BoundedValue::new(0, 0, 100);
    assert_eq!(val.get(), 0);
}

#[test]
fn bounded_value_at_max() {
    let val = BoundedValue::new(100, 0, 100);
    assert_eq!(val.get(), 100);
}
