//! WebGL2 Waterfall Display Component.
//!
//! Renders spectrum data as a scrolling waterfall display using WebGL2.
//! Uses texture streaming for efficient updates.

use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as GL, WebGlProgram, WebGlShader, WebGlTexture,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

/// Waterfall display width in pixels (FFT bins).
pub const WATERFALL_WIDTH: usize = 512;

/// Waterfall display height in pixels (history rows).
pub const WATERFALL_HEIGHT: usize = 256;

/// Vertex shader source for textured quad.
const VERTEX_SHADER_SRC: &str = r#"#version 300 es
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;

out vec2 v_texcoord;

void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
    v_texcoord = a_texcoord;
}
"#;

/// Fragment shader source with color palette mapping.
const FRAGMENT_SHADER_SRC: &str = r#"#version 300 es
precision mediump float;

in vec2 v_texcoord;
out vec4 fragColor;

uniform sampler2D u_texture;
uniform float u_row_offset;

// Color palette: black -> blue -> cyan -> green -> yellow -> red -> white
vec3 colormap(float value) {
    float v = clamp(value, 0.0, 1.0);

    if (v < 0.2) {
        // Black to blue
        float t = v / 0.2;
        return vec3(0.0, 0.0, t);
    } else if (v < 0.4) {
        // Blue to cyan
        float t = (v - 0.2) / 0.2;
        return vec3(0.0, t, 1.0);
    } else if (v < 0.6) {
        // Cyan to green
        float t = (v - 0.4) / 0.2;
        return vec3(0.0, 1.0, 1.0 - t);
    } else if (v < 0.8) {
        // Green to yellow
        float t = (v - 0.6) / 0.2;
        return vec3(t, 1.0, 0.0);
    } else {
        // Yellow to white
        float t = (v - 0.8) / 0.2;
        return vec3(1.0, 1.0 - t * 0.5, t);
    }
}

void main() {
    // Apply circular buffer offset for scrolling
    vec2 tc = v_texcoord;
    tc.y = fract(tc.y + u_row_offset);

    float intensity = texture(u_texture, tc).r;
    vec3 color = colormap(intensity);
    fragColor = vec4(color, 1.0);
}
"#;

/// WebGL waterfall renderer state.
pub struct WaterfallRenderer {
    gl: GL,
    program: WebGlProgram,
    vao: WebGlVertexArrayObject,
    texture: WebGlTexture,
    u_row_offset: WebGlUniformLocation,
    texture_data: Vec<u8>,
    current_row: usize,
}

impl WaterfallRenderer {
    /// Create a new waterfall renderer from a canvas element.
    pub fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let gl = canvas
            .get_context("webgl2")?
            .ok_or("Failed to get WebGL2 context")?
            .dyn_into::<GL>()?;

        // Set viewport
        gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);

        // Compile shaders
        let vert_shader = compile_shader(&gl, GL::VERTEX_SHADER, VERTEX_SHADER_SRC)?;
        let frag_shader = compile_shader(&gl, GL::FRAGMENT_SHADER, FRAGMENT_SHADER_SRC)?;
        let program = link_program(&gl, &vert_shader, &frag_shader)?;

        gl.use_program(Some(&program));

        // Get uniform locations
        let u_row_offset = gl
            .get_uniform_location(&program, "u_row_offset")
            .ok_or("Failed to get u_row_offset location")?;

        // Create VAO with fullscreen quad
        let vao = create_fullscreen_quad(&gl)?;

        // Create texture for waterfall data
        let texture = gl.create_texture().ok_or("Failed to create texture")?;
        gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::REPEAT as i32);

        // Initialize texture with zeros
        let texture_data = vec![0u8; WATERFALL_WIDTH * WATERFALL_HEIGHT];
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            GL::TEXTURE_2D,
            0,
            GL::R8 as i32,
            WATERFALL_WIDTH as i32,
            WATERFALL_HEIGHT as i32,
            0,
            GL::RED,
            GL::UNSIGNED_BYTE,
            Some(&texture_data),
        )?;

        Ok(Self {
            gl,
            program,
            vao,
            texture,
            u_row_offset,
            texture_data,
            current_row: 0,
        })
    }

    /// Push a new spectrum row to the waterfall.
    ///
    /// # Arguments
    /// * `spectrum` - Spectrum data, expected to be `WATERFALL_WIDTH` values in range 0.0-1.0
    pub fn push_row(&mut self, spectrum: &[f32]) {
        // Convert spectrum to u8 and copy to texture data
        let row_start = self.current_row * WATERFALL_WIDTH;
        for (i, &val) in spectrum.iter().take(WATERFALL_WIDTH).enumerate() {
            self.texture_data[row_start + i] = (val.clamp(0.0, 1.0) * 255.0) as u8;
        }

        // Update texture row
        self.gl.bind_texture(GL::TEXTURE_2D, Some(&self.texture));
        let _ = self
            .gl
            .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
                GL::TEXTURE_2D,
                0,
                0,
                self.current_row as i32,
                WATERFALL_WIDTH as i32,
                1,
                GL::RED,
                GL::UNSIGNED_BYTE,
                Some(&self.texture_data[row_start..row_start + WATERFALL_WIDTH]),
            );

        // Advance row (circular buffer)
        self.current_row = (self.current_row + 1) % WATERFALL_HEIGHT;
    }

    /// Render the waterfall display.
    pub fn render(&self) {
        self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
        self.gl.clear(GL::COLOR_BUFFER_BIT);

        self.gl.use_program(Some(&self.program));
        self.gl.bind_vertex_array(Some(&self.vao));
        self.gl.bind_texture(GL::TEXTURE_2D, Some(&self.texture));

        // Set row offset for circular buffer scrolling
        let row_offset = self.current_row as f32 / WATERFALL_HEIGHT as f32;
        self.gl.uniform1f(Some(&self.u_row_offset), row_offset);

        self.gl.draw_arrays(GL::TRIANGLE_STRIP, 0, 4);
    }

    /// Clear the waterfall display.
    pub fn clear(&mut self) {
        self.texture_data.fill(0);
        self.current_row = 0;

        self.gl.bind_texture(GL::TEXTURE_2D, Some(&self.texture));
        let _ = self
            .gl
            .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
                GL::TEXTURE_2D,
                0,
                0,
                0,
                WATERFALL_WIDTH as i32,
                WATERFALL_HEIGHT as i32,
                GL::RED,
                GL::UNSIGNED_BYTE,
                Some(&self.texture_data),
            );
    }
}

/// Compile a WebGL shader.
fn compile_shader(gl: &GL, shader_type: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or("Failed to create shader")?;

    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown shader error".to_string()))
    }
}

/// Link a WebGL program from vertex and fragment shaders.
fn link_program(gl: &GL, vert: &WebGlShader, frag: &WebGlShader) -> Result<WebGlProgram, String> {
    let program = gl.create_program().ok_or("Failed to create program")?;

    gl.attach_shader(&program, vert);
    gl.attach_shader(&program, frag);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| "Unknown linker error".to_string()))
    }
}

/// Create a fullscreen quad VAO.
fn create_fullscreen_quad(gl: &GL) -> Result<WebGlVertexArrayObject, JsValue> {
    let vao = gl.create_vertex_array().ok_or("Failed to create VAO")?;
    gl.bind_vertex_array(Some(&vao));

    // Positions: fullscreen quad in clip space
    // Texcoords: UV coordinates
    #[rustfmt::skip]
    let vertices: [f32; 16] = [
        // Position   // Texcoord
        -1.0, -1.0,   0.0, 1.0,  // Bottom-left
         1.0, -1.0,   1.0, 1.0,  // Bottom-right
        -1.0,  1.0,   0.0, 0.0,  // Top-left
         1.0,  1.0,   1.0, 0.0,  // Top-right
    ];

    let buffer = gl.create_buffer().ok_or("Failed to create buffer")?;
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buffer));

    unsafe {
        let vert_array = js_sys::Float32Array::view(&vertices);
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &vert_array, GL::STATIC_DRAW);
    }

    // Position attribute (location 0)
    gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 16, 0);
    gl.enable_vertex_attrib_array(0);

    // Texcoord attribute (location 1)
    gl.vertex_attrib_pointer_with_i32(1, 2, GL::FLOAT, false, 16, 8);
    gl.enable_vertex_attrib_array(1);

    Ok(vao)
}

/// Leptos Waterfall component.
#[component]
pub fn Waterfall(
    /// Width of the canvas in pixels
    #[prop(default = WATERFALL_WIDTH)]
    width: usize,
    /// Height of the canvas in pixels
    #[prop(default = WATERFALL_HEIGHT)]
    height: usize,
    /// Signal providing spectrum data (Vec<f32> of normalized values)
    spectrum: ReadSignal<Vec<f32>>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();
    let renderer: StoredValue<Option<WaterfallRenderer>> = store_value(None);

    // Initialize WebGL on mount
    create_effect(move |_| {
        if let Some(canvas) = canvas_ref.get() {
            let canvas_el: &HtmlCanvasElement = &canvas;
            canvas_el.set_width(width as u32);
            canvas_el.set_height(height as u32);

            match WaterfallRenderer::new(canvas_el) {
                Ok(r) => {
                    renderer.set_value(Some(r));
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Waterfall init error: {:?}", e).into());
                }
            }
        }
    });

    // Update waterfall when spectrum changes
    create_effect(move |_| {
        let data = spectrum.get();
        renderer.update_value(|r| {
            if let Some(ref mut renderer) = r {
                renderer.push_row(&data);
                renderer.render();
            }
        });
    });

    view! {
        <canvas
            node_ref=canvas_ref
            class="waterfall-canvas"
            style="display: block; image-rendering: pixelated;"
        />
    }
}
