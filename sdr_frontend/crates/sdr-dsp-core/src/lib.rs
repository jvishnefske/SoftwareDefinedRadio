//! SDR DSP Core Library
//!
//! Platform-agnostic DSP primitives for software-defined radio processing.
//! This crate is `no_std` compatible for use in both embedded and WASM targets.
//!
//! # Modules
//!
//! - [`types`] - Core types: IqSample, SignalMetrics
//! - [`filter`] - Digital filters: Biquad, FIR, DC blocker
//! - [`oscillator`] - Signal generators: NCO, quadrature oscillator
//! - [`agc`] - Automatic gain control and S-meter
//! - [`spectrum`] - Spectrum analysis: sliding DFT, waterfall data

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

pub mod agc;
pub mod filter;
pub mod oscillator;
pub mod spectrum;
pub mod types;

// Re-export commonly used types
pub use agc::{Agc, AgcConfig, SMeter};
pub use filter::{Biquad, BiquadCoeffs, DcBlocker};
pub use oscillator::{CostasLoop, Nco, QuadratureOscillator};
pub use spectrum::{FftSpectrum, SlidingDft, SpectrumBin, SpectrumConfig, WaterfallRow};
pub use types::{IqSample, SignalMetrics};
