//! SDR Transceiver Firmware Library
//!
//! This library provides the core functionality for an STM32G474-based
//! Software Defined Radio (SDR) transceiver. The design follows the
//! uSDX/truSDX architecture with a Class-E H-bridge power amplifier
//! and quadrature sampling detector.
//!
//! # Architecture
//!
//! The firmware is organized in layers:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    APPLICATION LAYER                         │
//! │  Radio Control  │  UI Manager  │  CAT Protocol               │
//! ├─────────────────────────────────────────────────────────────┤
//! │                      DSP LAYER                               │
//! │  Filters  │  Modulation (SSB/CW/AM)  │  Audio Processing     │
//! ├─────────────────────────────────────────────────────────────┤
//! │                   HAL / DRIVER LAYER                         │
//! │  ADC  │  DAC  │  I2C  │  SPI  │  USB  │  GPIO                │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    RTOS / SCHEDULER                          │
//! │           embassy-rs (async/await executor)                  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Design Principles
//!
//! - **Immutable-by-default**: State transitions return new instances
//! - **Type-driven design**: Custom types enforce invariants at compile time
//! - **No unsafe in application code**: All unsafe isolated in HAL/FFI layers
//! - **Functional core, imperative shell**: Pure logic separated from I/O
//! - **Explicit error handling**: All fallible operations return `Result`

#![cfg_attr(feature = "embedded", no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

// Re-export dependencies needed by applications (only in embedded mode)
#[cfg(feature = "embedded")]
pub use embassy_executor;
#[cfg(feature = "embedded")]
pub use embassy_stm32;
#[cfg(feature = "embedded")]
pub use embassy_time;
#[cfg(feature = "embedded")]
pub use embassy_usb;

/// Hardware Abstraction Layer
///
/// Provides safe abstractions over STM32G474 peripherals.
#[cfg(feature = "embedded")]
pub mod hal;

/// Peripheral Drivers
///
/// High-level drivers for external ICs (Si5351, display, etc.)
#[cfg(feature = "embedded")]
pub mod drivers;

/// Digital Signal Processing
///
/// Filters, oscillators, modulation/demodulation algorithms.
pub mod dsp;

/// Radio Control Logic
///
/// State machines and business logic for radio operation.
pub mod radio;

/// Power Management
///
/// Battery monitoring, USB-PD, thermal management.
pub mod power;

/// User Interface
///
/// Display rendering, menu system, input handling.
#[cfg(feature = "embedded")]
pub mod ui;

/// USB Subsystem
///
/// CDC ACM for CAT control, USB Audio for IQ streaming.
#[cfg(feature = "embedded")]
pub mod usb;

/// Communication Protocols
///
/// CAT command parser, IQ data formatting.
pub mod protocol;

/// Shared types used across modules
pub mod types;

/// System configuration and constants
pub mod config;

/// Prelude module for common imports
#[cfg(feature = "embedded")]
pub mod prelude {
    //! Convenient re-exports for common types and traits.

    pub use crate::config::*;
    pub use crate::types::*;

    // Common traits
    pub use embedded_hal::digital::OutputPin;
    pub use embedded_hal_async::i2c::I2c;

    // Embassy
    pub use embassy_time::{Duration, Instant, Timer};

    // Error handling
    pub use core::result::Result;

    // Logging
    pub use defmt::{debug, error, info, trace, warn};
}
