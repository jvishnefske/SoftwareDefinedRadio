//! Web Audio API integration for SDR processing.
//!
//! Handles AudioContext creation, AudioWorklet loading, and
//! data transfer between the audio thread and UI.

use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{AudioContext, AudioWorkletNode, AudioWorkletNodeOptions};

use crate::state::AppContext;

/// Audio pipeline manager.
///
/// Manages the Web Audio API components and data flow.
pub struct AudioPipeline {
    ctx: Option<AudioContext>,
    worklet_node: Option<AudioWorkletNode>,
}

impl AudioPipeline {
    /// Create a new audio pipeline (not yet started).
    pub fn new() -> Self {
        Self {
            ctx: None,
            worklet_node: None,
        }
    }

    /// Start the audio pipeline.
    ///
    /// This will:
    /// 1. Create an AudioContext
    /// 2. Load the AudioWorklet processor
    /// 3. Connect to audio input (microphone/line-in for IQ)
    /// 4. Start processing
    pub async fn start(&mut self) -> Result<(), JsValue> {
        // Create AudioContext
        let ctx = AudioContext::new()?;

        // Load the AudioWorklet processor module
        let worklet = ctx.audio_worklet()?;
        let promise = worklet.add_module("/worklet/processor.js")?;
        wasm_bindgen_futures::JsFuture::from(promise).await?;

        // Create AudioWorkletNode options
        let options = AudioWorkletNodeOptions::new();
        options.set_number_of_inputs(1);
        options.set_number_of_outputs(1);
        options.set_output_channel_count(&js_sys::Array::of1(&2.into())); // Stereo output

        // Create the AudioWorkletNode
        let node = AudioWorkletNode::new_with_options(&ctx, "sdr-dsp-processor", &options)?;

        // Get audio input (stereo for I/Q)
        let navigator = web_sys::window()
            .ok_or("No window")?
            .navigator();

        let media_devices = navigator.media_devices()?;

        // Request stereo audio input
        let constraints = web_sys::MediaStreamConstraints::new();
        let audio_constraints = js_sys::Object::new();
        js_sys::Reflect::set(&audio_constraints, &"channelCount".into(), &2.into())?;
        js_sys::Reflect::set(&audio_constraints, &"echoCancellation".into(), &false.into())?;
        js_sys::Reflect::set(&audio_constraints, &"noiseSuppression".into(), &false.into())?;
        js_sys::Reflect::set(&audio_constraints, &"autoGainControl".into(), &false.into())?;
        constraints.set_audio(&audio_constraints.into());

        let promise = media_devices.get_user_media_with_constraints(&constraints)?;
        let stream = wasm_bindgen_futures::JsFuture::from(promise)
            .await?
            .dyn_into::<web_sys::MediaStream>()?;

        // Create source from input stream
        let source = ctx.create_media_stream_source(&stream)?;

        // Connect: source -> worklet -> destination
        source.connect_with_audio_node(&node)?;
        node.connect_with_audio_node(&ctx.destination())?;

        // Resume audio context (required by browser autoplay policy)
        let resume_promise = ctx.resume()?;
        wasm_bindgen_futures::JsFuture::from(resume_promise).await?;

        self.ctx = Some(ctx);
        self.worklet_node = Some(node);

        Ok(())
    }

    /// Stop the audio pipeline.
    pub fn stop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            let _ = ctx.close();
        }
        self.worklet_node = None;
    }

    /// Check if the pipeline is running.
    pub fn is_running(&self) -> bool {
        self.ctx.is_some()
    }

    /// Get the AudioWorkletNode for message passing.
    pub fn worklet_node(&self) -> Option<&AudioWorkletNode> {
        self.worklet_node.as_ref()
    }

    /// Send a message to the AudioWorklet.
    pub fn send_message(&self, message: &JsValue) -> Result<(), JsValue> {
        if let Some(node) = &self.worklet_node {
            node.port()?.post_message(message)?;
        }
        Ok(())
    }

    /// Set the operating mode.
    pub fn set_mode(&self, mode: u8) -> Result<(), JsValue> {
        let msg = js_sys::Object::new();
        js_sys::Reflect::set(&msg, &"type".into(), &"setMode".into())?;
        js_sys::Reflect::set(&msg, &"mode".into(), &mode.into())?;
        self.send_message(&msg.into())
    }

    /// Set the frequency offset.
    pub fn set_frequency_offset(&self, offset_hz: f32) -> Result<(), JsValue> {
        let msg = js_sys::Object::new();
        js_sys::Reflect::set(&msg, &"type".into(), &"setFrequency".into())?;
        js_sys::Reflect::set(&msg, &"frequency".into(), &offset_hz.into())?;
        self.send_message(&msg.into())
    }

    /// Set filter bandwidth.
    pub fn set_bandwidth(&self, bandwidth_hz: f32) -> Result<(), JsValue> {
        let msg = js_sys::Object::new();
        js_sys::Reflect::set(&msg, &"type".into(), &"setBandwidth".into())?;
        js_sys::Reflect::set(&msg, &"bandwidth".into(), &bandwidth_hz.into())?;
        self.send_message(&msg.into())
    }
}

impl Default for AudioPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Create an effect that manages the audio pipeline based on app state.
pub fn create_audio_effect(app_ctx: AppContext) {
    let pipeline = store_value(AudioPipeline::new());

    // Clone for each effect
    let ctx_for_audio = app_ctx.clone();
    let ctx_for_mode = app_ctx.clone();
    let ctx_for_bandwidth = app_ctx;

    // Effect to start/stop audio based on audio_running signal
    create_effect(move |_| {
        let should_run = ctx_for_audio.audio_running.get();
        let ctx = ctx_for_audio.clone();

        if should_run {
            // Start audio
            let ctx_inner = ctx.clone();
            spawn_local(async move {
                let mut new_pipeline = AudioPipeline::new();
                match new_pipeline.start().await {
                    Ok(()) => {
                        web_sys::console::log_1(&"Audio pipeline started".into());
                        // Set up message handler for spectrum data
                        if let Some(node) = new_pipeline.worklet_node() {
                            if let Ok(port) = node.port() {
                                let ctx_msg = ctx_inner.clone();
                                let onmessage = Closure::wrap(Box::new(move |ev: web_sys::MessageEvent| {
                                    handle_worklet_message(&ctx_msg, ev);
                                }) as Box<dyn FnMut(_)>);
                                port.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                                onmessage.forget(); // Leak the closure (it lives for the pipeline lifetime)
                            }
                        }
                        pipeline.set_value(new_pipeline);
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to start audio: {:?}", e).into());
                        ctx_inner.audio_running.set(false);
                    }
                }
            });
        } else {
            // Stop audio
            pipeline.update_value(|p| {
                if p.is_running() {
                    p.stop();
                    web_sys::console::log_1(&"Audio pipeline stopped".into());
                }
            });
        }
    });

    // Effect to update mode when it changes
    create_effect(move |_| {
        let mode = ctx_for_mode.mode.get();
        pipeline.with_value(|p| {
            if p.is_running() {
                let _ = p.set_mode(mode.code());
            }
        });
    });

    // Effect to update bandwidth when it changes
    create_effect(move |_| {
        let bw = ctx_for_bandwidth.bandwidth.get();
        pipeline.with_value(|p| {
            if p.is_running() {
                let _ = p.set_bandwidth(bw);
            }
        });
    });
}

/// Handle messages from the AudioWorklet.
fn handle_worklet_message(ctx: &AppContext, ev: web_sys::MessageEvent) {
    let data = ev.data();

    // Check message type
    if let Ok(obj) = data.dyn_into::<js_sys::Object>() {
        if let Ok(msg_type) = js_sys::Reflect::get(&obj, &"type".into()) {
            let type_str = msg_type.as_string().unwrap_or_default();

            match type_str.as_str() {
                "spectrum" => {
                    // Spectrum data from worklet
                    if let Ok(spectrum_val) = js_sys::Reflect::get(&obj, &"data".into()) {
                        if let Ok(array) = spectrum_val.dyn_into::<js_sys::Float32Array>() {
                            let mut spectrum = vec![0.0f32; array.length() as usize];
                            array.copy_to(&mut spectrum);
                            ctx.spectrum.set(spectrum);
                        }
                    }
                }
                "smeter" => {
                    // S-meter value
                    if let Ok(val) = js_sys::Reflect::get(&obj, &"value".into()) {
                        if let Some(v) = val.as_f64() {
                            ctx.smeter.set(v as f32);
                        }
                    }
                }
                "decoded" => {
                    // Decoded text from digital mode
                    if let Ok(text) = js_sys::Reflect::get(&obj, &"text".into()) {
                        if let Some(s) = text.as_string() {
                            ctx.rx_text.update(|t| t.push_str(&s));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
