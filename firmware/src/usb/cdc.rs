//! USB CDC ACM (Serial) Implementation
//!
//! Provides virtual serial port for CAT control.

use embassy_usb::class::cdc_acm::State;
use heapless::Vec;

use crate::config::CAT_BUFFER_SIZE;

/// CDC ACM state
pub struct CdcState<'d> {
    state: State<'d>,
}

impl<'d> Default for CdcState<'d> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'d> CdcState<'d> {
    /// Create new CDC state
    #[must_use] 
    pub fn new() -> Self {
        Self {
            state: State::new(),
        }
    }

    /// Get reference to state for class creation
    pub fn state_mut(&mut self) -> &mut State<'d> {
        &mut self.state
    }
}

/// CDC read buffer
pub struct CdcReadBuffer {
    buffer: [u8; CAT_BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
}

impl CdcReadBuffer {
    /// Create a new read buffer
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0; CAT_BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
        }
    }

    /// Push data into buffer
    pub fn push(&mut self, data: &[u8]) -> usize {
        let mut written = 0;
        for &byte in data {
            if self.write_pos < CAT_BUFFER_SIZE {
                self.buffer[self.write_pos] = byte;
                self.write_pos += 1;
                written += 1;
            }
        }
        written
    }

    /// Read a line (up to newline or CR)
    pub fn read_line(&mut self) -> Option<Vec<u8, CAT_BUFFER_SIZE>> {
        // Find newline
        let newline_pos = self.buffer[self.read_pos..self.write_pos]
            .iter()
            .position(|&b| b == b'\n' || b == b'\r');

        if let Some(pos) = newline_pos {
            let end = self.read_pos + pos;
            let mut line = Vec::new();
            for i in self.read_pos..end {
                let _ = line.push(self.buffer[i]);
            }

            // Skip the newline character(s)
            self.read_pos = end + 1;
            while self.read_pos < self.write_pos
                && (self.buffer[self.read_pos] == b'\n' || self.buffer[self.read_pos] == b'\r')
            {
                self.read_pos += 1;
            }

            // Compact buffer if needed
            if self.read_pos >= CAT_BUFFER_SIZE / 2 {
                self.compact();
            }

            Some(line)
        } else {
            None
        }
    }

    /// Compact the buffer
    fn compact(&mut self) {
        if self.read_pos > 0 {
            let remaining = self.write_pos - self.read_pos;
            self.buffer.copy_within(self.read_pos..self.write_pos, 0);
            self.read_pos = 0;
            self.write_pos = remaining;
        }
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
    }

    /// Get available bytes
    #[must_use]
    pub const fn available(&self) -> usize {
        self.write_pos - self.read_pos
    }

    /// Get free space
    #[must_use]
    pub const fn free(&self) -> usize {
        CAT_BUFFER_SIZE - self.write_pos
    }
}

impl Default for CdcReadBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// CDC write buffer
pub struct CdcWriteBuffer {
    buffer: [u8; CAT_BUFFER_SIZE],
    len: usize,
}

impl CdcWriteBuffer {
    /// Create a new write buffer
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [0; CAT_BUFFER_SIZE],
            len: 0,
        }
    }

    /// Write data to buffer
    pub fn write(&mut self, data: &[u8]) -> usize {
        let space = CAT_BUFFER_SIZE - self.len;
        let to_write = data.len().min(space);
        self.buffer[self.len..self.len + to_write].copy_from_slice(&data[..to_write]);
        self.len += to_write;
        to_write
    }

    /// Write a string
    pub fn write_str(&mut self, s: &str) -> usize {
        self.write(s.as_bytes())
    }

    /// Write with newline
    pub fn writeln(&mut self, data: &[u8]) -> usize {
        let written = self.write(data);
        if self.len < CAT_BUFFER_SIZE {
            self.buffer[self.len] = b'\r';
            self.len += 1;
        }
        if self.len < CAT_BUFFER_SIZE {
            self.buffer[self.len] = b'\n';
            self.len += 1;
        }
        written + 2
    }

    /// Get buffer contents
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Get used length
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for CdcWriteBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Line coding (baud rate, etc.)
#[derive(Clone, Copy, Debug)]
pub struct LineCoding {
    /// Baud rate
    pub baud_rate: u32,
    /// Data bits (5, 6, 7, 8)
    pub data_bits: u8,
    /// Stop bits (1, 1.5, 2)
    pub stop_bits: StopBits,
    /// Parity
    pub parity: Parity,
}

impl Default for LineCoding {
    fn default() -> Self {
        Self {
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: StopBits::One,
            parity: Parity::None,
        }
    }
}

/// Stop bits configuration
#[derive(Clone, Copy, Debug, Default)]
pub enum StopBits {
    /// One stop bit
    #[default]
    One,
    /// One and a half stop bits
    OnePointFive,
    /// Two stop bits
    Two,
}

/// Parity configuration
#[derive(Clone, Copy, Debug, Default)]
pub enum Parity {
    /// No parity
    #[default]
    None,
    /// Odd parity
    Odd,
    /// Even parity
    Even,
    /// Mark parity
    Mark,
    /// Space parity
    Space,
}

/// DTR/RTS control signals
#[derive(Clone, Copy, Debug, Default)]
pub struct ControlSignals {
    /// Data Terminal Ready
    pub dtr: bool,
    /// Request To Send
    pub rts: bool,
}

impl ControlSignals {
    /// Check if host is connected (DTR set)
    #[must_use]
    pub const fn connected(&self) -> bool {
        self.dtr
    }
}

/// USB device descriptor strings
pub struct UsbStrings {
    /// Manufacturer name
    pub manufacturer: &'static str,
    /// Product name
    pub product: &'static str,
    /// Serial number
    pub serial: &'static str,
}

impl Default for UsbStrings {
    fn default() -> Self {
        Self {
            manufacturer: "SDR Project",
            product: "SDR Transceiver",
            serial: "0001",
        }
    }
}

/// USB device info for descriptor
#[derive(Clone, Copy, Debug)]
pub struct UsbDeviceInfo {
    /// Vendor ID
    pub vid: u16,
    /// Product ID
    pub pid: u16,
    /// Device release number
    pub device_release: u16,
}

impl Default for UsbDeviceInfo {
    fn default() -> Self {
        Self {
            vid: crate::config::USB_VID,
            pid: crate::config::USB_PID,
            device_release: 0x0100,
        }
    }
}

impl defmt::Format for UsbDeviceInfo {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "USB({:04X}:{:04X})", self.vid, self.pid);
    }
}
