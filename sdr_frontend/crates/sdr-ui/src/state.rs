//! Application state management.

use crate::components::RadioMode;
use leptos::*;

/// Radio state: frequency, mode, transmit status.
#[derive(Clone, Debug)]
pub struct RadioState {
    /// Current frequency in Hz
    pub frequency: u64,
    /// Current operating mode
    pub mode: RadioMode,
    /// Transmit state
    pub transmitting: bool,
    /// Filter bandwidth in Hz
    pub bandwidth: f32,
}

impl Default for RadioState {
    fn default() -> Self {
        Self {
            frequency: 14_070_000, // 20m PSK31 calling frequency
            mode: RadioMode::Usb,
            transmitting: false,
            bandwidth: 2700.0,
        }
    }
}

/// Display state: spectrum, waterfall, S-meter.
#[derive(Clone, Debug, Default)]
pub struct DisplayState {
    /// Current spectrum data (normalized 0.0-1.0)
    pub spectrum: Vec<f32>,
    /// S-meter value (0.0 = S0, 1.0 = S9)
    pub smeter: f32,
}

/// Digital decoder state.
#[derive(Clone, Debug, Default)]
pub struct DecoderState {
    /// Received text buffer
    pub rx_text: String,
    /// Transmit text buffer
    pub tx_buffer: String,
    /// AFC frequency offset in Hz
    pub afc_offset: f32,
    /// AFC enabled
    pub afc_enabled: bool,
}

/// Application context providing global state.
#[derive(Clone)]
pub struct AppContext {
    /// Radio state signals
    pub frequency: RwSignal<u64>,
    pub mode: RwSignal<RadioMode>,
    pub transmitting: RwSignal<bool>,
    pub bandwidth: RwSignal<f32>,

    /// Display state signals
    pub spectrum: RwSignal<Vec<f32>>,
    pub smeter: RwSignal<f32>,

    /// Decoder state signals
    pub rx_text: RwSignal<String>,
    pub tx_buffer: RwSignal<String>,
    pub afc_offset: RwSignal<f32>,
    pub afc_enabled: RwSignal<bool>,

    /// Audio pipeline running
    pub audio_running: RwSignal<bool>,
}

impl AppContext {
    /// Create new application context with default values.
    pub fn new() -> Self {
        let radio = RadioState::default();
        let display = DisplayState::default();
        let decoder = DecoderState::default();

        Self {
            frequency: create_rw_signal(radio.frequency),
            mode: create_rw_signal(radio.mode),
            transmitting: create_rw_signal(radio.transmitting),
            bandwidth: create_rw_signal(radio.bandwidth),
            spectrum: create_rw_signal(display.spectrum),
            smeter: create_rw_signal(display.smeter),
            rx_text: create_rw_signal(decoder.rx_text),
            tx_buffer: create_rw_signal(decoder.tx_buffer),
            afc_offset: create_rw_signal(decoder.afc_offset),
            afc_enabled: create_rw_signal(decoder.afc_enabled),
            audio_running: create_rw_signal(false),
        }
    }
}

impl Default for AppContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Provide application context to component tree.
pub fn provide_app_context() -> AppContext {
    let ctx = AppContext::new();
    provide_context(ctx.clone());
    ctx
}

/// Use application context from component tree.
pub fn use_app_context() -> AppContext {
    expect_context::<AppContext>()
}
