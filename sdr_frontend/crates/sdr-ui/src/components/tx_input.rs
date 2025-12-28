//! TX Input Component.
//!
//! Text input for digital mode transmission.

use leptos::*;

/// TX input component for digital modes.
#[component]
pub fn TxInput(
    /// Signal for TX text buffer
    tx_buffer: RwSignal<String>,
    /// Callback when transmit is requested
    on_transmit: Callback<()>,
    /// Whether currently transmitting
    is_transmitting: ReadSignal<bool>,
) -> impl IntoView {
    let input_ref = create_node_ref::<leptos::html::Textarea>();

    let handle_input = move |ev: web_sys::Event| {
        let target = event_target::<web_sys::HtmlTextAreaElement>(&ev);
        tx_buffer.set(target.value());
    };

    let handle_submit = move |_: web_sys::MouseEvent| {
        if !is_transmitting.get() && !tx_buffer.get().is_empty() {
            on_transmit.call(());
        }
    };

    let do_transmit = move || {
        if !is_transmitting.get() && !tx_buffer.get().is_empty() {
            on_transmit.call(());
        }
    };

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        // Ctrl+Enter to transmit
        if ev.ctrl_key() && ev.key() == "Enter" {
            ev.prevent_default();
            do_transmit();
        }
    };

    let button_text = move || {
        if is_transmitting.get() {
            "TX..."
        } else {
            "TX"
        }
    };

    view! {
        <div class="tx-input-container">
            <textarea
                node_ref=input_ref
                class="tx-input"
                placeholder="Enter text to transmit..."
                prop:value=move || tx_buffer.get()
                on:input=handle_input
                on:keydown=handle_keydown
                disabled=is_transmitting
            />
            <button
                class="tx-button"
                class:transmitting=is_transmitting
                on:click=handle_submit
                disabled=move || is_transmitting.get() || tx_buffer.get().is_empty()
            >
                {button_text}
            </button>
        </div>
    }
}
