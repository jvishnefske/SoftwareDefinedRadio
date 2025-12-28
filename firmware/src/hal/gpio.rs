//! GPIO Abstractions
//!
//! Type-safe GPIO pin wrappers for the SDR transceiver.
//! Provides semantic meaning to pins through the type system.

use embassy_stm32::gpio::{Input, Output};

/// Status LED state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LedState {
    /// LED is off
    #[default]
    Off,
    /// LED is on
    On,
}

impl LedState {
    /// Toggle the LED state
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::Off => Self::On,
            Self::On => Self::Off,
        }
    }
}

impl defmt::Format for LedState {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Off => defmt::write!(f, "OFF"),
            Self::On => defmt::write!(f, "ON"),
        }
    }
}

/// Status LED driver
pub struct StatusLed<'d> {
    pin: Output<'d>,
    state: LedState,
}

impl<'d> StatusLed<'d> {
    /// Create a new status LED (initially off)
    #[must_use] 
    pub fn new(pin: Output<'d>) -> Self {
        Self {
            pin,
            state: LedState::Off,
        }
    }

    /// Turn LED on
    pub fn on(&mut self) {
        self.pin.set_high();
        self.state = LedState::On;
    }

    /// Turn LED off
    pub fn off(&mut self) {
        self.pin.set_low();
        self.state = LedState::Off;
    }

    /// Toggle LED state
    pub fn toggle(&mut self) {
        match self.state {
            LedState::Off => self.on(),
            LedState::On => self.off(),
        }
    }

    /// Get current state
    #[must_use]
    pub const fn state(&self) -> LedState {
        self.state
    }
}

/// PTT (Push-to-Talk) input
pub struct PttInput<'d> {
    pin: Input<'d>,
}

impl<'d> PttInput<'d> {
    /// Create a new PTT input (active low with internal pull-up)
    #[must_use] 
    pub fn new(pin: Input<'d>) -> Self {
        Self { pin }
    }

    /// Check if PTT is pressed (active low)
    #[must_use]
    pub fn is_pressed(&self) -> bool {
        self.pin.is_low()
    }

    /// Check if PTT is released
    #[must_use]
    pub fn is_released(&self) -> bool {
        self.pin.is_high()
    }
}

/// T/R relay control
pub struct TrRelay<'d> {
    pin: Output<'d>,
    is_tx: bool,
}

impl<'d> TrRelay<'d> {
    /// Create T/R relay control (starts in RX mode)
    #[must_use] 
    pub fn new(pin: Output<'d>) -> Self {
        Self { pin, is_tx: false }
    }

    /// Switch to transmit mode
    pub fn set_tx(&mut self) {
        self.pin.set_high();
        self.is_tx = true;
    }

    /// Switch to receive mode
    pub fn set_rx(&mut self) {
        self.pin.set_low();
        self.is_tx = false;
    }

    /// Check if in transmit mode
    #[must_use]
    pub const fn is_tx(&self) -> bool {
        self.is_tx
    }
}

/// LPF (Low Pass Filter) bank selector
///
/// Controls the 5-bank LPF using 3 GPIO pins for binary selection.
pub struct LpfSelector<'d> {
    sel0: Output<'d>,
    sel1: Output<'d>,
    sel2: Output<'d>,
    current_bank: u8,
}

impl<'d> LpfSelector<'d> {
    /// Create LPF selector (initially selects bank 0)
    #[must_use] 
    pub fn new(sel0: Output<'d>, sel1: Output<'d>, sel2: Output<'d>) -> Self {
        let mut selector = Self {
            sel0,
            sel1,
            sel2,
            current_bank: 0,
        };
        selector.select(0);
        selector
    }

    /// Select LPF bank (0-4)
    pub fn select(&mut self, bank: u8) {
        let bank = bank.min(4);
        self.current_bank = bank;

        // Set GPIO pins based on binary representation
        if bank & 0x01 != 0 {
            self.sel0.set_high();
        } else {
            self.sel0.set_low();
        }

        if bank & 0x02 != 0 {
            self.sel1.set_high();
        } else {
            self.sel1.set_low();
        }

        if bank & 0x04 != 0 {
            self.sel2.set_high();
        } else {
            self.sel2.set_low();
        }
    }

    /// Get currently selected bank
    #[must_use]
    pub const fn current(&self) -> u8 {
        self.current_bank
    }

    /// Select bank for frequency
    pub fn select_for_frequency(&mut self, freq_hz: u32) {
        let bank = match freq_hz {
            0..=5_000_000 => 0,         // 80m
            5_000_001..=8_000_000 => 1, // 40m
            8_000_001..=16_000_000 => 2, // 30m/20m
            16_000_001..=19_000_000 => 3, // 17m
            _ => 4,                      // 15m
        };
        self.select(bank);
    }
}

/// Encoder button state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonState {
    /// Button is released
    Released,
    /// Button is pressed
    Pressed,
}

impl defmt::Format for ButtonState {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Released => defmt::write!(f, "Released"),
            Self::Pressed => defmt::write!(f, "Pressed"),
        }
    }
}

/// Encoder push button with debouncing
pub struct EncoderButton<'d> {
    pin: Input<'d>,
    state: ButtonState,
    last_raw: bool,
    debounce_count: u8,
}

impl<'d> EncoderButton<'d> {
    /// Required consecutive reads for debounce
    const DEBOUNCE_THRESHOLD: u8 = 3;

    /// Create encoder button (active low with pull-up)
    #[must_use] 
    pub fn new(pin: Input<'d>) -> Self {
        Self {
            pin,
            state: ButtonState::Released,
            last_raw: true,
            debounce_count: 0,
        }
    }

    /// Update button state (call periodically)
    /// Returns true if state changed
    pub fn update(&mut self) -> bool {
        let current = self.pin.is_low();

        if current == self.last_raw {
            if self.debounce_count < Self::DEBOUNCE_THRESHOLD {
                self.debounce_count += 1;
            }
        } else {
            self.debounce_count = 0;
            self.last_raw = current;
        }

        if self.debounce_count >= Self::DEBOUNCE_THRESHOLD {
            let new_state = if current {
                ButtonState::Pressed
            } else {
                ButtonState::Released
            };

            if new_state != self.state {
                self.state = new_state;
                return true;
            }
        }

        false
    }

    /// Get current state
    #[must_use]
    pub const fn state(&self) -> ButtonState {
        self.state
    }

    /// Check if pressed
    #[must_use]
    pub const fn is_pressed(&self) -> bool {
        matches!(self.state, ButtonState::Pressed)
    }
}
