//! SDR Web UI - Leptos-based frontend.
//!
//! Provides a browser-based interface for SDR operation including:
//! - Waterfall display
//! - Frequency control
//! - Digital mode decoding
//! - Radio control via Web Serial

pub mod app;
pub mod audio;
pub mod components;
pub mod serial;
pub mod state;

pub use app::App;
pub use audio::{create_audio_effect, AudioPipeline};
pub use serial::{CatControlPanel, CatProtocol, CatSerial};
