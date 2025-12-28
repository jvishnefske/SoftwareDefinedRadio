//! Mode Selector Component.
//!
//! Dropdown/button group for selecting operating mode.

use leptos::*;

/// Radio operating modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioMode {
    /// Lower Sideband
    Lsb,
    /// Upper Sideband
    Usb,
    /// Continuous Wave
    Cw,
    /// Amplitude Modulation
    Am,
    /// Frequency Modulation
    Fm,
    /// PSK31 Digital Mode
    Psk31,
    /// RTTY Digital Mode
    Rtty,
}

impl RadioMode {
    /// Get display name for the mode.
    pub fn name(&self) -> &'static str {
        match self {
            RadioMode::Lsb => "LSB",
            RadioMode::Usb => "USB",
            RadioMode::Cw => "CW",
            RadioMode::Am => "AM",
            RadioMode::Fm => "FM",
            RadioMode::Psk31 => "PSK31",
            RadioMode::Rtty => "RTTY",
        }
    }

    /// Get mode code for DSP processor.
    pub fn code(&self) -> u8 {
        match self {
            RadioMode::Lsb => 0,
            RadioMode::Usb => 1,
            RadioMode::Cw => 2,
            RadioMode::Am => 3,
            RadioMode::Fm => 4,
            RadioMode::Psk31 => 1, // Uses USB with digital decoder
            RadioMode::Rtty => 1,  // Uses USB with digital decoder
        }
    }

    /// Check if this is a digital mode.
    pub fn is_digital(&self) -> bool {
        matches!(self, RadioMode::Psk31 | RadioMode::Rtty)
    }

    /// All available modes.
    pub fn all() -> &'static [RadioMode] {
        &[
            RadioMode::Lsb,
            RadioMode::Usb,
            RadioMode::Cw,
            RadioMode::Am,
            RadioMode::Fm,
            RadioMode::Psk31,
            RadioMode::Rtty,
        ]
    }
}

/// Mode selector component.
#[component]
pub fn ModeSelector(
    /// Current mode
    mode: ReadSignal<RadioMode>,
    /// Callback when mode changes
    on_change: Callback<RadioMode>,
) -> impl IntoView {
    view! {
        <div class="mode-selector">
            {RadioMode::all()
                .iter()
                .map(|&m| {
                    let is_selected = move || mode.get() == m;
                    view! {
                        <button
                            class="mode-button"
                            class:selected=is_selected
                            class:digital=m.is_digital()
                            on:click=move |_| on_change.call(m)
                        >
                            {m.name()}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}
