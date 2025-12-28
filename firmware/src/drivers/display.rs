//! OLED Display Driver
//!
//! Provides display rendering for the SDR transceiver UI.
//! Uses the SSD1306 controller with I2C interface.

use crate::hal::i2c::{I2cAddress, I2cBus, I2cResult};
use crate::types::{Band, Frequency, Mode, TuningStep, TxRxState};
use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use heapless::String;

/// Display width in pixels
pub const DISPLAY_WIDTH: u32 = 128;

/// Display height in pixels
pub const DISPLAY_HEIGHT: u32 = 64;

/// SSD1306 commands
mod cmd {
    pub const SET_CONTRAST: u8 = 0x81;
    pub const DISPLAY_ALL_ON_RESUME: u8 = 0xA4;
    pub const NORMAL_DISPLAY: u8 = 0xA6;
    pub const INVERT_DISPLAY: u8 = 0xA7;
    pub const DISPLAY_OFF: u8 = 0xAE;
    pub const DISPLAY_ON: u8 = 0xAF;
    pub const SET_DISPLAY_OFFSET: u8 = 0xD3;
    pub const SET_COM_PINS: u8 = 0xDA;
    pub const SET_VCOM_DETECT: u8 = 0xDB;
    pub const SET_DISPLAY_CLOCK_DIV: u8 = 0xD5;
    pub const SET_PRECHARGE: u8 = 0xD9;
    pub const SET_MULTIPLEX: u8 = 0xA8;
    pub const SET_START_LINE: u8 = 0x40;
    pub const MEMORY_MODE: u8 = 0x20;
    pub const COLUMN_ADDR: u8 = 0x21;
    pub const PAGE_ADDR: u8 = 0x22;
    pub const COM_SCAN_DEC: u8 = 0xC8;
    pub const SEG_REMAP: u8 = 0xA0;
    pub const CHARGE_PUMP: u8 = 0x8D;
}

/// Display buffer (1 bit per pixel)
pub struct DisplayBuffer {
    /// Pixel data (128x64 / 8 = 1024 bytes)
    buffer: [u8; 1024],
}

impl DisplayBuffer {
    /// Create a new empty display buffer
    #[must_use]
    pub const fn new() -> Self {
        Self { buffer: [0; 1024] }
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.fill(0);
    }

    /// Set a pixel
    pub fn set_pixel(&mut self, x: u32, y: u32, on: bool) {
        if x >= DISPLAY_WIDTH || y >= DISPLAY_HEIGHT {
            return;
        }

        let byte_idx = (y / 8 * DISPLAY_WIDTH + x) as usize;
        let bit = 1 << (y % 8);

        if on {
            self.buffer[byte_idx] |= bit;
        } else {
            self.buffer[byte_idx] &= !bit;
        }
    }

    /// Get the raw buffer
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }
}

impl Default for DisplayBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement `DrawTarget` for embedded-graphics
impl DrawTarget for DisplayBuffer {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            if coord.x >= 0
                && coord.x < DISPLAY_WIDTH as i32
                && coord.y >= 0
                && coord.y < DISPLAY_HEIGHT as i32
            {
                self.set_pixel(coord.x as u32, coord.y as u32, color.is_on());
            }
        }
        Ok(())
    }
}

impl OriginDimensions for DisplayBuffer {
    fn size(&self) -> Size {
        Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT)
    }
}

/// OLED display driver
pub struct Display<'d> {
    bus: I2cBus<'d>,
    buffer: DisplayBuffer,
}

impl<'d> Display<'d> {
    /// Create a new display driver
    #[must_use] 
    pub fn new(i2c: I2c<'d, Async>) -> Self {
        Self {
            bus: I2cBus::new(i2c),
            buffer: DisplayBuffer::new(),
        }
    }

    /// Initialize the display
    pub async fn init(&mut self) -> I2cResult<()> {
        // Initialization sequence for SSD1306 128x64
        let init_cmds = [
            cmd::DISPLAY_OFF,
            cmd::SET_DISPLAY_CLOCK_DIV,
            0x80, // Default clock
            cmd::SET_MULTIPLEX,
            0x3F, // 64 lines
            cmd::SET_DISPLAY_OFFSET,
            0x00,
            cmd::SET_START_LINE,
            cmd::CHARGE_PUMP,
            0x14, // Enable charge pump
            cmd::MEMORY_MODE,
            0x00, // Horizontal addressing
            cmd::SEG_REMAP | 0x01,
            cmd::COM_SCAN_DEC,
            cmd::SET_COM_PINS,
            0x12,
            cmd::SET_CONTRAST,
            0xCF,
            cmd::SET_PRECHARGE,
            0xF1,
            cmd::SET_VCOM_DETECT,
            0x40,
            cmd::DISPLAY_ALL_ON_RESUME,
            cmd::NORMAL_DISPLAY,
            cmd::DISPLAY_ON,
        ];

        for &c in &init_cmds {
            self.send_command(c).await?;
        }

        // Clear the display
        self.buffer.clear();
        self.flush().await?;

        Ok(())
    }

    /// Send a command to the display
    async fn send_command(&mut self, cmd: u8) -> I2cResult<()> {
        self.bus.write(I2cAddress::SSD1306, &[0x00, cmd]).await
    }

    /// Flush the buffer to the display
    pub async fn flush(&mut self) -> I2cResult<()> {
        // Set column address
        self.send_command(cmd::COLUMN_ADDR).await?;
        self.send_command(0).await?;
        self.send_command(127).await?;

        // Set page address
        self.send_command(cmd::PAGE_ADDR).await?;
        self.send_command(0).await?;
        self.send_command(7).await?;

        // Send data in chunks (I2C buffer limit)
        let data = self.buffer.as_bytes();
        for chunk in data.chunks(32) {
            let mut buf = [0u8; 33];
            buf[0] = 0x40; // Data mode
            buf[1..=chunk.len()].copy_from_slice(chunk);
            self.bus
                .write(I2cAddress::SSD1306, &buf[..=chunk.len()])
                .await?;
        }

        Ok(())
    }

    /// Get mutable access to the buffer for drawing
    #[must_use]
    pub fn buffer_mut(&mut self) -> &mut DisplayBuffer {
        &mut self.buffer
    }

    /// Clear the display
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Set display contrast
    pub async fn set_contrast(&mut self, contrast: u8) -> I2cResult<()> {
        self.send_command(cmd::SET_CONTRAST).await?;
        self.send_command(contrast).await
    }

    /// Invert display colors
    pub async fn invert(&mut self, invert: bool) -> I2cResult<()> {
        if invert {
            self.send_command(cmd::INVERT_DISPLAY).await
        } else {
            self.send_command(cmd::NORMAL_DISPLAY).await
        }
    }
}

/// Radio status display renderer
pub struct StatusRenderer;

impl StatusRenderer {
    /// Render frequency display (large, centered)
    pub fn render_frequency(buffer: &mut DisplayBuffer, freq: Frequency) {
        let mhz = freq.as_hz() / 1_000_000;
        let khz = (freq.as_hz() % 1_000_000) / 1000;
        let hz = freq.as_hz() % 1000;

        let mut s: String<16> = String::new();
        core::fmt::write(&mut s, format_args!("{mhz:2}.{khz:03}.{hz:03}")).ok();

        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let _ = Text::with_baseline(&s, Point::new(20, 20), style, Baseline::Top)
            .draw(buffer);
    }

    /// Render mode indicator
    pub fn render_mode(buffer: &mut DisplayBuffer, mode: Mode) {
        let mode_str = match mode {
            Mode::Lsb => "LSB",
            Mode::Usb => "USB",
            Mode::Cw => "CW",
            Mode::CwR => "CWR",
            Mode::Am => "AM",
            Mode::Fm => "FM",
        };

        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let _ = Text::with_baseline(mode_str, Point::new(100, 0), style, Baseline::Top)
            .draw(buffer);
    }

    /// Render band indicator
    pub fn render_band(buffer: &mut DisplayBuffer, band: Option<Band>) {
        let band_str = match band {
            Some(Band::M80) => "80m",
            Some(Band::M40) => "40m",
            Some(Band::M30) => "30m",
            Some(Band::M20) => "20m",
            Some(Band::M17) => "17m",
            Some(Band::M15) => "15m",
            None => "---",
        };

        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let _ = Text::with_baseline(band_str, Point::new(0, 0), style, Baseline::Top)
            .draw(buffer);
    }

    /// Render TX/RX indicator
    pub fn render_txrx(buffer: &mut DisplayBuffer, state: TxRxState) {
        let (text, invert) = match state {
            TxRxState::Rx => ("RX", false),
            TxRxState::Tx => ("TX", true),
            TxRxState::Switching => ("--", false),
        };

        if invert {
            // Draw inverted (white on black box)
            let rect = Rectangle::new(Point::new(50, 0), Size::new(20, 12));
            let _ = rect
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(buffer);

            let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::Off);
            let _ = Text::with_baseline(text, Point::new(52, 1), style, Baseline::Top)
                .draw(buffer);
        } else {
            let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
            let _ = Text::with_baseline(text, Point::new(52, 0), style, Baseline::Top)
                .draw(buffer);
        }
    }

    /// Render tuning step indicator
    pub fn render_step(buffer: &mut DisplayBuffer, step: TuningStep) {
        let step_str = match step {
            TuningStep::Hz1 => "1Hz",
            TuningStep::Hz10 => "10Hz",
            TuningStep::Hz100 => "100Hz",
            TuningStep::KHz1 => "1kHz",
            TuningStep::KHz10 => "10kHz",
            TuningStep::KHz100 => "100k",
            TuningStep::MHz1 => "1MHz",
        };

        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let _ = Text::with_baseline(step_str, Point::new(0, 54), style, Baseline::Top)
            .draw(buffer);
    }

    /// Render S-meter bar
    pub fn render_smeter(buffer: &mut DisplayBuffer, level: u8) {
        let y = 40;
        let max_width = 100;
        let bar_width = (u32::from(level) * max_width / 100) as i32;

        // Draw outline
        let outline = Rectangle::new(Point::new(14, y), Size::new(max_width, 8));
        let _ = outline
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(buffer);

        // Draw fill
        if bar_width > 0 {
            let fill = Rectangle::new(Point::new(14, y), Size::new(bar_width as u32, 8));
            let _ = fill
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(buffer);
        }

        // Draw S-meter label
        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let _ = Text::with_baseline("S", Point::new(2, y), style, Baseline::Top).draw(buffer);
    }

    /// Render SWR indicator
    pub fn render_swr(buffer: &mut DisplayBuffer, swr: f32) {
        let mut s: String<8> = String::new();
        if swr > 9.9 {
            core::fmt::write(&mut s, format_args!("SWR:HI")).ok();
        } else {
            let whole = swr as u32;
            let frac = ((swr - whole as f32) * 10.0) as u32;
            core::fmt::write(&mut s, format_args!("SWR:{whole}.{frac}")).ok();
        }

        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let _ = Text::with_baseline(&s, Point::new(70, 54), style, Baseline::Top)
            .draw(buffer);
    }
}
