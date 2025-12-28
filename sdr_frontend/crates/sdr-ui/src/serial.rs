//! Web Serial API integration for CAT control.
//!
//! Provides serial communication with amateur radio transceivers
//! using the Kenwood TS-2000/TS-480 protocol.
//!
//! Note: Web Serial API requires browser support and HTTPS context.
//! The API is still experimental and may not be available in all browsers.

use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::state::AppContext;

/// CAT (Computer Aided Transceiver) command protocol.
///
/// Implements a subset of the Kenwood TS-2000 protocol.
pub struct CatProtocol;

impl CatProtocol {
    /// Create frequency query command.
    pub fn frequency_query() -> &'static str {
        "FA;"
    }

    /// Create frequency set command.
    pub fn frequency_set(hz: u64) -> String {
        format!("FA{:011};", hz)
    }

    /// Create mode query command.
    pub fn mode_query() -> &'static str {
        "MD;"
    }

    /// Create mode set command.
    pub fn mode_set(mode: u8) -> String {
        format!("MD{};", mode)
    }

    /// Create PTT command (0 = RX, 1 = TX).
    pub fn ptt(transmit: bool) -> String {
        format!("TX{};", if transmit { 1 } else { 0 })
    }

    /// Parse frequency response (FA00014070000;).
    pub fn parse_frequency(response: &str) -> Option<u64> {
        if response.starts_with("FA") && response.ends_with(';') {
            let freq_str = &response[2..response.len() - 1];
            freq_str.parse().ok()
        } else {
            None
        }
    }

    /// Parse mode response (MD1;).
    pub fn parse_mode(response: &str) -> Option<u8> {
        if response.starts_with("MD") && response.ends_with(';') {
            let mode_str = &response[2..response.len() - 1];
            mode_str.parse().ok()
        } else {
            None
        }
    }
}

/// Web Serial port wrapper for CAT control.
///
/// Note: This is a stub implementation. The Web Serial API
/// requires unstable web-sys features that may not be available.
pub struct CatSerial {
    connected: bool,
    port: Option<js_sys::Object>,
}

impl CatSerial {
    /// Create a new CAT serial handler.
    pub fn new() -> Self {
        Self {
            connected: false,
            port: None,
        }
    }

    /// Check if Web Serial API is available.
    pub fn is_available() -> bool {
        if let Some(window) = web_sys::window() {
            let navigator = window.navigator();
            // Check if navigator.serial exists
            js_sys::Reflect::has(&navigator, &"serial".into()).unwrap_or(false)
        } else {
            false
        }
    }

    /// Request and open a serial port.
    pub async fn connect(&mut self, baud_rate: u32) -> Result<(), JsValue> {
        if !Self::is_available() {
            return Err("Web Serial API not available".into());
        }

        let window = web_sys::window().ok_or("No window")?;
        let navigator = window.navigator();

        // Get serial object from navigator
        let serial = js_sys::Reflect::get(&navigator, &"serial".into())?;

        // Call requestPort()
        let request_port = js_sys::Reflect::get(&serial, &"requestPort".into())?;
        let request_port_fn = request_port.dyn_into::<js_sys::Function>()?;
        let promise = request_port_fn.call0(&serial)?;
        let port = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await?;

        // Call port.open({ baudRate })
        let options = js_sys::Object::new();
        js_sys::Reflect::set(&options, &"baudRate".into(), &baud_rate.into())?;

        let open_fn = js_sys::Reflect::get(&port, &"open".into())?
            .dyn_into::<js_sys::Function>()?;
        let open_promise = open_fn.call1(&port, &options)?;
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(open_promise)).await?;

        self.port = Some(port.dyn_into::<js_sys::Object>()?);
        self.connected = true;

        Ok(())
    }

    /// Disconnect from the serial port.
    pub async fn disconnect(&mut self) -> Result<(), JsValue> {
        if let Some(port) = self.port.take() {
            let close_fn = js_sys::Reflect::get(&port, &"close".into())?
                .dyn_into::<js_sys::Function>()?;
            let close_promise = close_fn.call0(&port)?;
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(close_promise)).await?;
        }
        self.connected = false;
        Ok(())
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Send a CAT command.
    pub async fn send(&self, command: &str) -> Result<(), JsValue> {
        if let Some(port) = &self.port {
            // Get writable stream
            let writable = js_sys::Reflect::get(port, &"writable".into())?;
            let get_writer = js_sys::Reflect::get(&writable, &"getWriter".into())?
                .dyn_into::<js_sys::Function>()?;
            let writer = get_writer.call0(&writable)?;

            // Write data
            let data = js_sys::Uint8Array::from(command.as_bytes());
            let write_fn = js_sys::Reflect::get(&writer, &"write".into())?
                .dyn_into::<js_sys::Function>()?;
            let write_promise = write_fn.call1(&writer, &data)?;
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(write_promise)).await?;

            // Release lock
            let release_fn = js_sys::Reflect::get(&writer, &"releaseLock".into())?
                .dyn_into::<js_sys::Function>()?;
            release_fn.call0(&writer)?;
        }
        Ok(())
    }

    /// Read a response (until ';' terminator).
    pub async fn read_response(&self) -> Result<String, JsValue> {
        let mut response = String::new();

        if let Some(port) = &self.port {
            // Get readable stream
            let readable = js_sys::Reflect::get(port, &"readable".into())?;
            let get_reader = js_sys::Reflect::get(&readable, &"getReader".into())?
                .dyn_into::<js_sys::Function>()?;
            let reader = get_reader.call0(&readable)?;

            loop {
                let read_fn = js_sys::Reflect::get(&reader, &"read".into())?
                    .dyn_into::<js_sys::Function>()?;
                let read_promise = read_fn.call0(&reader)?;
                let result =
                    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(read_promise))
                        .await?;

                let done = js_sys::Reflect::get(&result, &"done".into())?
                    .as_bool()
                    .unwrap_or(true);

                if done {
                    break;
                }

                if let Ok(value) = js_sys::Reflect::get(&result, &"value".into()) {
                    if let Ok(array) = value.dyn_into::<js_sys::Uint8Array>() {
                        let mut buf = vec![0u8; array.length() as usize];
                        array.copy_to(&mut buf);
                        let chunk = String::from_utf8_lossy(&buf);
                        response.push_str(&chunk);

                        if response.ends_with(';') {
                            break;
                        }
                    }
                }
            }

            // Release lock
            let release_fn = js_sys::Reflect::get(&reader, &"releaseLock".into())?
                .dyn_into::<js_sys::Function>()?;
            release_fn.call0(&reader)?;
        }

        Ok(response)
    }

    /// Query and return current frequency.
    pub async fn get_frequency(&self) -> Result<Option<u64>, JsValue> {
        self.send(CatProtocol::frequency_query()).await?;
        let response = self.read_response().await?;
        Ok(CatProtocol::parse_frequency(&response))
    }

    /// Set frequency.
    pub async fn set_frequency(&self, hz: u64) -> Result<(), JsValue> {
        let cmd = CatProtocol::frequency_set(hz);
        self.send(&cmd).await
    }

    /// Query and return current mode.
    pub async fn get_mode(&self) -> Result<Option<u8>, JsValue> {
        self.send(CatProtocol::mode_query()).await?;
        let response = self.read_response().await?;
        Ok(CatProtocol::parse_mode(&response))
    }

    /// Set mode.
    pub async fn set_mode(&self, mode: u8) -> Result<(), JsValue> {
        let cmd = CatProtocol::mode_set(mode);
        self.send(&cmd).await
    }

    /// Set PTT state.
    pub async fn set_ptt(&self, transmit: bool) -> Result<(), JsValue> {
        let cmd = CatProtocol::ptt(transmit);
        self.send(&cmd).await
    }
}

impl Default for CatSerial {
    fn default() -> Self {
        Self::new()
    }
}

/// Leptos component for CAT serial controls.
#[component]
pub fn CatControlPanel(ctx: AppContext) -> impl IntoView {
    let connected = create_rw_signal(false);
    let status = create_rw_signal("Disconnected".to_string());
    let available = CatSerial::is_available();

    // Clone ctx for each closure
    let ctx_connect = ctx.clone();
    let ctx_sync = ctx;

    let connect = move |_: web_sys::MouseEvent| {
        let _ctx = ctx_connect.clone();
        spawn_local(async move {
            let mut serial = CatSerial::new();
            match serial.connect(9600).await {
                Ok(()) => {
                    connected.set(true);
                    status.set("Connected".to_string());
                    web_sys::console::log_1(&"CAT serial connected".into());
                }
                Err(e) => {
                    status.set(format!("Error: {:?}", e));
                    web_sys::console::error_1(&format!("CAT connect error: {:?}", e).into());
                }
            }
        });
    };

    let disconnect = move |_: web_sys::MouseEvent| {
        spawn_local(async move {
            let mut serial = CatSerial::new();
            if let Err(e) = serial.disconnect().await {
                web_sys::console::error_1(&format!("CAT disconnect error: {:?}", e).into());
            }
            connected.set(false);
            status.set("Disconnected".to_string());
        });
    };

    let sync_from_radio = move |_: web_sys::MouseEvent| {
        let ctx = ctx_sync.clone();
        spawn_local(async move {
            let serial = CatSerial::new();
            if let Ok(Some(freq)) = serial.get_frequency().await {
                ctx.frequency.set(freq);
            }
        });
    };

    view! {
        <div class="cat-control-panel">
            <h3>"CAT Control"</h3>
            {if available {
                view! {
                    <div class="cat-status">
                        <span class="status-indicator" class:connected=connected />
                        <span class="status-text">{move || status.get()}</span>
                    </div>
                    <div class="cat-buttons">
                        <button
                            on:click=connect
                            disabled=connected
                        >
                            "Connect"
                        </button>
                        <button
                            on:click=disconnect
                            disabled=move || !connected.get()
                        >
                            "Disconnect"
                        </button>
                        <button
                            on:click=sync_from_radio
                            disabled=move || !connected.get()
                        >
                            "Sync"
                        </button>
                    </div>
                }.into_view()
            } else {
                view! {
                    <div class="cat-unavailable">
                        <p>"Web Serial API not available."</p>
                        <p>"Use Chrome/Edge with HTTPS."</p>
                    </div>
                }.into_view()
            }}
        </div>
    }
}
