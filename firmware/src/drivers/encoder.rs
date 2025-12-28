//! Rotary Encoder Driver
//!
//! Handles rotary encoder input for tuning and menu navigation.
//! Supports quadrature decoding with debouncing.

use crate::hal::gpio::{ButtonState, EncoderButton};
use embassy_stm32::gpio::Input;

/// Encoder rotation direction
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Clockwise rotation (increment)
    Clockwise,
    /// Counter-clockwise rotation (decrement)
    CounterClockwise,
}

impl defmt::Format for Direction {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Clockwise => defmt::write!(f, "CW"),
            Self::CounterClockwise => defmt::write!(f, "CCW"),
        }
    }
}

/// Encoder event
#[derive(Clone, Copy, Debug)]
pub enum EncoderEvent {
    /// Encoder rotated
    Rotate {
        /// Direction of rotation
        direction: Direction,
        /// Number of steps
        steps: u32,
    },
    /// Button pressed
    ButtonPress,
    /// Button released
    ButtonRelease,
    /// Button held for long press
    LongPress,
}

impl defmt::Format for EncoderEvent {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Rotate { direction, steps } => {
                defmt::write!(f, "Rotate({}, {})", direction, steps);
            }
            Self::ButtonPress => defmt::write!(f, "Press"),
            Self::ButtonRelease => defmt::write!(f, "Release"),
            Self::LongPress => defmt::write!(f, "LongPress"),
        }
    }
}

/// Encoder state machine states
#[derive(Clone, Copy, Debug, Default)]
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

/// Quadrature encoder decoder using state machine
pub struct QuadratureDecoder {
    state: EncoderState,
    last_a: bool,
    last_b: bool,
}

impl QuadratureDecoder {
    /// Create a new quadrature decoder
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: EncoderState::Idle,
            last_a: false,
            last_b: false,
        }
    }

    /// Update with new A/B pin states, returns direction if step completed
    pub fn update(&mut self, a: bool, b: bool) -> Option<Direction> {
        use EncoderState::{Idle, CwStart, CcwStart, CwNext, CwFinal, CcwNext, CcwFinal};

        // Only process on state change
        if a == self.last_a && b == self.last_b {
            return None;
        }

        self.last_a = a;
        self.last_b = b;

        // State machine for quadrature decoding
        let (new_state, result) = match (self.state, a, b) {
            // Idle state - detect start of rotation
            (Idle, false, true) => (CwStart, None),
            (Idle, true, false) => (CcwStart, None),

            // Clockwise sequence: 00 -> 01 -> 11 -> 10 -> 00
            (CwStart, true, true) => (CwNext, None),
            (CwNext, true, false) => (CwFinal, None),
            (CwFinal, false, false) => (Idle, Some(Direction::Clockwise)),

            // Counter-clockwise sequence: 00 -> 10 -> 11 -> 01 -> 00
            (CcwStart, true, true) => (CcwNext, None),
            (CcwNext, false, true) => (CcwFinal, None),
            (CcwFinal, false, false) => (Idle, Some(Direction::CounterClockwise)),

            // Invalid transition - reset to idle
            _ => (Idle, None),
        };

        self.state = new_state;
        result
    }

    /// Reset the decoder state
    pub fn reset(&mut self) {
        self.state = EncoderState::Idle;
    }
}

impl Default for QuadratureDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Acceleration curve for fast tuning
#[derive(Clone, Copy, Debug)]
pub struct AccelerationCurve {
    /// Time threshold for acceleration in milliseconds
    threshold_ms: u32,
    /// Multiplier when accelerating
    multiplier: u32,
    /// Last event timestamp
    last_event_ms: u32,
    /// Accumulated steps for acceleration
    step_count: u32,
}

impl AccelerationCurve {
    /// Create a new acceleration curve
    #[must_use]
    pub const fn new(threshold_ms: u32, multiplier: u32) -> Self {
        Self {
            threshold_ms,
            multiplier,
            last_event_ms: 0,
            step_count: 0,
        }
    }

    /// Process a step and return effective step count
    pub fn process(&mut self, current_ms: u32) -> u32 {
        let elapsed = current_ms.wrapping_sub(self.last_event_ms);
        self.last_event_ms = current_ms;

        if elapsed < self.threshold_ms {
            // Fast rotation - apply acceleration
            self.step_count = self.step_count.saturating_add(1).min(10);
            1 + (self.step_count * self.multiplier / 10)
        } else {
            // Slow rotation - reset acceleration
            self.step_count = 0;
            1
        }
    }

    /// Reset acceleration state
    pub fn reset(&mut self) {
        self.step_count = 0;
    }
}

impl Default for AccelerationCurve {
    fn default() -> Self {
        Self::new(50, 5) // Accelerate after 50ms, up to 5x
    }
}

/// Complete encoder driver with button
pub struct Encoder<'d> {
    /// A phase input
    a_pin: Input<'d>,
    /// B phase input
    b_pin: Input<'d>,
    /// Push button
    button: EncoderButton<'d>,
    /// Quadrature decoder
    decoder: QuadratureDecoder,
    /// Acceleration curve
    acceleration: AccelerationCurve,
    /// Button press start time for long press detection
    press_start_ms: Option<u32>,
    /// Long press threshold in milliseconds
    long_press_ms: u32,
    /// Whether long press was triggered
    long_press_triggered: bool,
}

impl<'d> Encoder<'d> {
    /// Default long press threshold
    pub const DEFAULT_LONG_PRESS_MS: u32 = 500;

    /// Create a new encoder driver
    #[must_use] 
    pub fn new(a_pin: Input<'d>, b_pin: Input<'d>, button: EncoderButton<'d>) -> Self {
        Self {
            a_pin,
            b_pin,
            button,
            decoder: QuadratureDecoder::new(),
            acceleration: AccelerationCurve::default(),
            press_start_ms: None,
            long_press_ms: Self::DEFAULT_LONG_PRESS_MS,
            long_press_triggered: false,
        }
    }

    /// Poll for encoder events (call periodically)
    pub fn poll(&mut self, current_ms: u32) -> Option<EncoderEvent> {
        // Check for rotation
        let a = self.a_pin.is_high();
        let b = self.b_pin.is_high();

        if let Some(direction) = self.decoder.update(a, b) {
            let steps = self.acceleration.process(current_ms);
            return Some(EncoderEvent::Rotate { direction, steps });
        }

        // Check for button events
        let button_changed = self.button.update();

        if button_changed {
            match self.button.state() {
                ButtonState::Pressed => {
                    self.press_start_ms = Some(current_ms);
                    self.long_press_triggered = false;
                    return Some(EncoderEvent::ButtonPress);
                }
                ButtonState::Released => {
                    self.press_start_ms = None;
                    if !self.long_press_triggered {
                        return Some(EncoderEvent::ButtonRelease);
                    }
                }
            }
        }

        // Check for long press
        if let Some(start) = self.press_start_ms {
            if !self.long_press_triggered {
                let held_ms = current_ms.wrapping_sub(start);
                if held_ms >= self.long_press_ms {
                    self.long_press_triggered = true;
                    return Some(EncoderEvent::LongPress);
                }
            }
        }

        None
    }

    /// Check if button is currently pressed
    #[must_use]
    pub fn is_pressed(&self) -> bool {
        self.button.is_pressed()
    }

    /// Set long press threshold
    pub fn set_long_press_ms(&mut self, ms: u32) {
        self.long_press_ms = ms;
    }

    /// Set acceleration parameters
    pub fn set_acceleration(&mut self, threshold_ms: u32, multiplier: u32) {
        self.acceleration = AccelerationCurve::new(threshold_ms, multiplier);
    }
}

/// Encoder value wrapper with min/max bounds
#[derive(Clone, Copy, Debug)]
pub struct BoundedValue<T> {
    value: T,
    min: T,
    max: T,
}

impl<T: Copy + Ord> BoundedValue<T> {
    /// Create a new bounded value
    #[must_use]
    pub const fn new(value: T, min: T, max: T) -> Self {
        Self { value, min, max }
    }

    /// Get current value
    #[must_use]
    pub const fn get(&self) -> T {
        self.value
    }

    /// Set value (clamped to bounds)
    pub fn set(&mut self, value: T) {
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
    /// Increment by amount
    pub fn increment(&mut self, amount: i32) {
        self.set(self.value.saturating_add(amount));
    }

    /// Decrement by amount
    pub fn decrement(&mut self, amount: i32) {
        self.set(self.value.saturating_sub(amount));
    }

    /// Handle encoder rotation
    pub fn handle_rotation(&mut self, direction: Direction, steps: u32) {
        let delta = steps as i32;
        match direction {
            Direction::Clockwise => self.increment(delta),
            Direction::CounterClockwise => self.decrement(delta),
        }
    }
}

impl BoundedValue<u32> {
    /// Increment by amount
    pub fn increment(&mut self, amount: u32) {
        self.set(self.value.saturating_add(amount));
    }

    /// Decrement by amount
    pub fn decrement(&mut self, amount: u32) {
        self.set(self.value.saturating_sub(amount));
    }

    /// Handle encoder rotation
    pub fn handle_rotation(&mut self, direction: Direction, steps: u32) {
        match direction {
            Direction::Clockwise => self.increment(steps),
            Direction::CounterClockwise => self.decrement(steps),
        }
    }
}
