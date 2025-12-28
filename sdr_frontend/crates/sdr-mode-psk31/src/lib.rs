//! PSK31 Digital Mode Decoder/Encoder
//!
//! Implements PSK31 (Phase Shift Keying, 31.25 baud) digital mode
//! for amateur radio text communication.
//!
//! # Features
//! - BPSK demodulation with Costas loop carrier tracking
//! - Varicode encoding/decoding
//! - AFC (Automatic Frequency Control)
//! - Signal quality metrics (IMD, SNR)

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod decoder;
pub mod encoder;
pub mod varicode;

pub use decoder::{Psk31Decoder, Psk31DecoderConfig};
pub use encoder::{Psk31Encoder, Psk31EncoderConfig};
pub use varicode::{VaricodeDecoder, VaricodeEncoder, VARICODE_TABLE};
