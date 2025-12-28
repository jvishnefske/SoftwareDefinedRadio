//! RX Text Display Component.
//!
//! Displays decoded digital mode text.

use leptos::*;

/// Maximum number of characters to display in the RX buffer.
const MAX_RX_CHARS: usize = 4096;

/// RX text display component for digital modes.
#[component]
pub fn RxTextDisplay(
    /// Signal containing received text
    text: ReadSignal<String>,
    /// Whether to auto-scroll to bottom
    #[prop(default = true)]
    auto_scroll: bool,
) -> impl IntoView {
    let container_ref = create_node_ref::<leptos::html::Div>();

    // Auto-scroll effect
    create_effect(move |_| {
        let _ = text.get();
        if auto_scroll {
            if let Some(container) = container_ref.get() {
                let el: &web_sys::Element = &container;
                el.set_scroll_top(el.scroll_height());
            }
        }
    });

    let display_text = move || {
        let t = text.get();
        if t.len() > MAX_RX_CHARS {
            t[t.len() - MAX_RX_CHARS..].to_string()
        } else {
            t
        }
    };

    view! {
        <div class="rx-text-container" node_ref=container_ref>
            <pre class="rx-text">{display_text}</pre>
        </div>
    }
}
