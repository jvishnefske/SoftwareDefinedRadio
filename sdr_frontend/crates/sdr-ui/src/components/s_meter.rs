//! S-Meter Component.
//!
//! Signal strength meter display.

use leptos::*;

/// S-meter display component.
#[component]
pub fn SMeterDisplay(
    /// S-meter value (0.0 = S0, 1.0 = S9, >1.0 = S9+)
    value: ReadSignal<f32>,
) -> impl IntoView {
    let s_reading = move || {
        let v = value.get();
        let s_units = v * 9.0;
        if s_units <= 9.0 {
            format!("S{}", s_units.round() as u8)
        } else {
            let over_db = ((s_units - 9.0) * 6.0).round() as i32;
            format!("S9+{}", over_db)
        }
    };

    let bar_width = move || {
        let v = value.get().clamp(0.0, 1.5);
        format!("{}%", (v / 1.5 * 100.0).round())
    };

    let bar_class = move || {
        let v = value.get();
        if v >= 1.0 {
            "s-meter-bar strong"
        } else if v >= 0.5 {
            "s-meter-bar medium"
        } else {
            "s-meter-bar weak"
        }
    };

    view! {
        <div class="s-meter">
            <div class="s-meter-scale">
                <span class="s1">"1"</span>
                <span class="s3">"3"</span>
                <span class="s5">"5"</span>
                <span class="s7">"7"</span>
                <span class="s9">"9"</span>
                <span class="s9plus">"+20"</span>
            </div>
            <div class="s-meter-bar-container">
                <div class=bar_class style:width=bar_width></div>
            </div>
            <div class="s-meter-reading">{s_reading}</div>
        </div>
    }
}
