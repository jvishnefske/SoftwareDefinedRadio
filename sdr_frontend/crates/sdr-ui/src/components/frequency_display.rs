//! Frequency Display Component.
//!
//! VFO display with digit tuning capability.

use leptos::*;

/// Format frequency in MHz with proper grouping.
fn format_frequency(hz: u64) -> String {
    let mhz = hz / 1_000_000;
    let khz = (hz % 1_000_000) / 1_000;
    let hz_rem = hz % 1_000;
    format!("{:>3}.{:03}.{:03}", mhz, khz, hz_rem)
}

/// Frequency display component with digit-based tuning.
#[component]
pub fn FrequencyDisplay(
    /// Current frequency in Hz
    frequency: ReadSignal<u64>,
    /// Callback when frequency changes
    on_change: Callback<u64>,
    /// Whether the display is active (can be tuned)
    #[prop(default = true)]
    active: bool,
) -> impl IntoView {
    let formatted = move || format_frequency(frequency.get());

    // Tuning step sizes for each digit position
    let step_sizes: [u64; 9] = [
        100_000_000, // 100 MHz
        10_000_000,  // 10 MHz
        1_000_000,   // 1 MHz
        100_000,     // 100 kHz
        10_000,      // 10 kHz
        1_000,       // 1 kHz
        100,         // 100 Hz
        10,          // 10 Hz
        1,           // 1 Hz
    ];

    let handle_digit_click = move |digit_index: usize, direction: i32| {
        if !active {
            return;
        }
        let current = frequency.get();
        let step = step_sizes.get(digit_index).copied().unwrap_or(1);
        let new_freq = if direction > 0 {
            current.saturating_add(step)
        } else {
            current.saturating_sub(step)
        };
        on_change.call(new_freq.clamp(100_000, 30_000_000_000)); // 100kHz to 30GHz
    };

    view! {
        <div class="frequency-display" class:active=active>
            <div class="frequency-digits">
                {move || {
                    let freq_str = formatted();
                    freq_str
                        .chars()
                        .enumerate()
                        .map(|(i, ch)| {
                            if ch == '.' {
                                view! { <span class="separator">"."</span> }.into_view()
                            } else {
                                let digit_idx = match i {
                                    0..=2 => i,
                                    4..=6 => i - 1,
                                    8..=10 => i - 2,
                                    _ => 0,
                                };
                                view! {
                                    <span
                                        class="digit"
                                        on:wheel=move |ev| {
                                            ev.prevent_default();
                                            let delta = if ev.delta_y() < 0.0 { 1 } else { -1 };
                                            handle_digit_click(digit_idx, delta);
                                        }
                                    >
                                        {ch.to_string()}
                                    </span>
                                }.into_view()
                            }
                        })
                        .collect_view()
                }}
            </div>
            <div class="frequency-unit">"MHz"</div>
        </div>
    }
}
