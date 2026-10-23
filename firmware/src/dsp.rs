//! Digital Signal Processing
//!
//! Provides DSP algorithms for the SDR transceiver including:
//! - FIR/IIR filters for audio processing
//! - Hilbert transform for SSB generation
//! - AGC (Automatic Gain Control)
//! - CW tone generation
//! - Audio processing chain

pub mod agc;
pub mod audio_chain;
pub mod filter;
pub mod filter_design;
pub mod modulation;
pub mod noise_reduction;
pub mod oscillator;
pub mod si5351_calc;
pub mod spectrum;
