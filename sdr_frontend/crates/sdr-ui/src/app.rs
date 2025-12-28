//! Main application component.

use leptos::*;

use crate::components::{
    FrequencyDisplay, ModeSelector, RadioMode, RxTextDisplay, SMeterDisplay, TxInput, Waterfall,
};
use crate::state::{provide_app_context, AppContext};

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    // Provide application context
    let ctx = provide_app_context();

    view! {
        <main class="sdr-app">
            <Header ctx=ctx.clone() />
            <div class="main-content">
                <div class="display-section">
                    <Waterfall
                        width=512
                        height=256
                        spectrum=ctx.spectrum.read_only()
                    />
                    <SpectrumInfo ctx=ctx.clone() />
                </div>
                <div class="control-section">
                    <DigitalModePanel ctx=ctx.clone() />
                </div>
            </div>
            <StatusBar ctx=ctx.clone() />
        </main>
    }
}

/// Application header with frequency and mode controls.
#[component]
fn Header(ctx: AppContext) -> impl IntoView {
    let on_freq_change = Callback::new(move |freq| {
        ctx.frequency.set(freq);
    });

    let on_mode_change = Callback::new(move |mode: RadioMode| {
        ctx.mode.set(mode);
    });

    view! {
        <header class="app-header">
            <FrequencyDisplay
                frequency=ctx.frequency.read_only()
                on_change=on_freq_change
            />
            <ModeSelector
                mode=ctx.mode.read_only()
                on_change=on_mode_change
            />
            <SMeterDisplay value=ctx.smeter.read_only() />
            <AudioControls ctx=ctx.clone() />
        </header>
    }
}

/// Audio start/stop controls.
#[component]
fn AudioControls(ctx: AppContext) -> impl IntoView {
    let toggle_audio = move |_| {
        let running = ctx.audio_running.get();
        ctx.audio_running.set(!running);
        // Audio pipeline start/stop will be handled by effect
    };

    let button_text = move || {
        if ctx.audio_running.get() {
            "Stop Audio"
        } else {
            "Start Audio"
        }
    };

    view! {
        <div class="audio-controls">
            <button
                class="audio-toggle"
                class:running=move || ctx.audio_running.get()
                on:click=toggle_audio
            >
                {button_text}
            </button>
        </div>
    }
}

/// Spectrum info display (frequency markers, etc).
#[component]
fn SpectrumInfo(ctx: AppContext) -> impl IntoView {
    let center_freq = move || {
        let freq = ctx.frequency.get();
        format!("{:.3} MHz", freq as f64 / 1_000_000.0)
    };

    let afc_display = move || {
        let offset = ctx.afc_offset.get();
        if ctx.afc_enabled.get() && offset.abs() > 1.0 {
            format!("AFC: {:+.0} Hz", offset)
        } else {
            String::new()
        }
    };

    view! {
        <div class="spectrum-info">
            <span class="center-freq">{center_freq}</span>
            <span class="afc-offset">{afc_display}</span>
        </div>
    }
}

/// Digital mode panel with RX/TX text areas.
#[component]
fn DigitalModePanel(ctx: AppContext) -> impl IntoView {
    let is_digital = move || ctx.mode.get().is_digital();

    let on_transmit = Callback::new(move |_| {
        ctx.transmitting.set(true);
        // TX will be handled by audio pipeline
    });

    view! {
        <div class="digital-mode-panel" class:hidden=move || !is_digital()>
            <div class="rx-section">
                <h3>"Receive"</h3>
                <RxTextDisplay text=ctx.rx_text.read_only() />
            </div>
            <div class="tx-section">
                <h3>"Transmit"</h3>
                <TxInput
                    tx_buffer=ctx.tx_buffer
                    on_transmit=on_transmit
                    is_transmitting=ctx.transmitting.read_only()
                />
            </div>
            <AfcControls ctx=ctx.clone() />
        </div>
    }
}

/// AFC (Automatic Frequency Control) controls.
#[component]
fn AfcControls(ctx: AppContext) -> impl IntoView {
    let toggle_afc = move |_| {
        ctx.afc_enabled.update(|v| *v = !*v);
    };

    view! {
        <div class="afc-controls">
            <label>
                <input
                    type="checkbox"
                    prop:checked=move || ctx.afc_enabled.get()
                    on:change=toggle_afc
                />
                "AFC"
            </label>
        </div>
    }
}

/// Status bar at bottom of application.
#[component]
fn StatusBar(ctx: AppContext) -> impl IntoView {
    let status_text = move || {
        if ctx.transmitting.get() {
            "TX"
        } else if ctx.audio_running.get() {
            "RX"
        } else {
            "Idle"
        }
    };

    let mode_text = move || ctx.mode.get().name();

    view! {
        <footer class="status-bar">
            <span class="status">{status_text}</span>
            <span class="mode">{mode_text}</span>
            <span class="version">"SDR Frontend v0.1.0"</span>
        </footer>
    }
}
