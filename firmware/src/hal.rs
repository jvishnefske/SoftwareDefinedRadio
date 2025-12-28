//! Hardware Abstraction Layer
//!
//! Provides safe abstractions over STM32G474 peripherals.
//! This module isolates hardware-specific code and provides
//! async interfaces for all peripheral operations.

pub mod adc;
pub mod dac;
pub mod gpio;
pub mod i2c;
pub mod pwm;
pub mod timer;
