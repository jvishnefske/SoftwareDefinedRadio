//! Buffer and Ring Buffer Tests
//!
//! Tests for circular buffers used in USB CDC and other I/O operations.
//! Run with: cargo test --target x86_64-unknown-linux-gnu --no-default-features --features std --test buffer_tests

/// Simple ring buffer for testing (mirrors USB CDC buffer logic)
struct RingBuffer<const N: usize> {
    buffer: [u8; N],
    read_pos: usize,
    write_pos: usize,
}

impl<const N: usize> RingBuffer<N> {
    fn new() -> Self {
        Self {
            buffer: [0; N],
            read_pos: 0,
            write_pos: 0,
        }
    }

    fn push(&mut self, data: &[u8]) -> usize {
        let mut written = 0;
        for &byte in data {
            if self.write_pos < N {
                self.buffer[self.write_pos] = byte;
                self.write_pos += 1;
                written += 1;
            }
        }
        written
    }

    fn read_line(&mut self) -> Option<Vec<u8>> {
        let newline_pos = self.buffer[self.read_pos..self.write_pos]
            .iter()
            .position(|&b| b == b'\n' || b == b'\r' || b == b';');

        if let Some(pos) = newline_pos {
            let end = self.read_pos + pos;
            let line: Vec<u8> = self.buffer[self.read_pos..end].to_vec();

            self.read_pos = end + 1;
            while self.read_pos < self.write_pos
                && (self.buffer[self.read_pos] == b'\n' || self.buffer[self.read_pos] == b'\r')
            {
                self.read_pos += 1;
            }

            if self.read_pos >= N / 2 {
                self.compact();
            }

            Some(line)
        } else {
            None
        }
    }

    fn compact(&mut self) {
        if self.read_pos > 0 {
            let remaining = self.write_pos - self.read_pos;
            self.buffer.copy_within(self.read_pos..self.write_pos, 0);
            self.read_pos = 0;
            self.write_pos = remaining;
        }
    }

    fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
    }

    fn available(&self) -> usize {
        self.write_pos - self.read_pos
    }

    fn free(&self) -> usize {
        N - self.write_pos
    }
}

/// Write buffer for output (mirrors CDC write buffer)
struct WriteBuffer<const N: usize> {
    buffer: [u8; N],
    len: usize,
}

impl<const N: usize> WriteBuffer<N> {
    fn new() -> Self {
        Self {
            buffer: [0; N],
            len: 0,
        }
    }

    fn write(&mut self, data: &[u8]) -> usize {
        let space = N - self.len;
        let to_write = data.len().min(space);
        self.buffer[self.len..self.len + to_write].copy_from_slice(&data[..to_write]);
        self.len += to_write;
        to_write
    }

    fn write_str(&mut self, s: &str) -> usize {
        self.write(s.as_bytes())
    }

    fn writeln(&mut self, data: &[u8]) -> usize {
        let written = self.write(data);
        if self.len < N {
            self.buffer[self.len] = b'\r';
            self.len += 1;
        }
        if self.len < N {
            self.buffer[self.len] = b'\n';
            self.len += 1;
        }
        written + 2
    }

    fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// =============================================================================
// Ring Buffer Tests
// =============================================================================

#[test]
fn ring_buffer_creation() {
    let buf: RingBuffer<64> = RingBuffer::new();
    assert_eq!(buf.available(), 0);
    assert_eq!(buf.free(), 64);
}

#[test]
fn ring_buffer_push_simple() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    let written = buf.push(b"hello");
    assert_eq!(written, 5);
    assert_eq!(buf.available(), 5);
}

#[test]
fn ring_buffer_push_multiple() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    buf.push(b"hello");
    buf.push(b" ");
    buf.push(b"world");
    assert_eq!(buf.available(), 11);
}

#[test]
fn ring_buffer_read_line_newline() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    buf.push(b"FA00007074000;\n");

    let line = buf.read_line();
    assert!(line.is_some());
    assert_eq!(line.unwrap(), b"FA00007074000");
}

#[test]
fn ring_buffer_read_line_cr() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    buf.push(b"ID;\r");

    let line = buf.read_line();
    assert!(line.is_some());
    assert_eq!(line.unwrap(), b"ID");
}

#[test]
fn ring_buffer_read_line_semicolon() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    buf.push(b"FA;FB;");

    let line1 = buf.read_line();
    assert!(line1.is_some());
    assert_eq!(line1.unwrap(), b"FA");

    let line2 = buf.read_line();
    assert!(line2.is_some());
    assert_eq!(line2.unwrap(), b"FB");
}

#[test]
fn ring_buffer_read_line_no_terminator() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    buf.push(b"incomplete");

    let line = buf.read_line();
    assert!(line.is_none());
}

#[test]
fn ring_buffer_read_line_empty() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    let line = buf.read_line();
    assert!(line.is_none());
}

#[test]
fn ring_buffer_multiple_lines() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    buf.push(b"line1\nline2\nline3\n");

    assert_eq!(buf.read_line().unwrap(), b"line1");
    assert_eq!(buf.read_line().unwrap(), b"line2");
    assert_eq!(buf.read_line().unwrap(), b"line3");
    assert!(buf.read_line().is_none());
}

#[test]
fn ring_buffer_clear() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    buf.push(b"some data");
    buf.clear();
    assert_eq!(buf.available(), 0);
}

#[test]
fn ring_buffer_compaction() {
    let mut buf: RingBuffer<64> = RingBuffer::new();

    // Fill buffer with enough data to trigger compaction
    // Compaction happens when read_pos >= N/2 (32 for 64-byte buffer)
    buf.push(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n"); // 32 bytes
    buf.push(b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n"); // 32 bytes (fills buffer)

    // Read first line to move read_pos past halfway
    let _line1 = buf.read_line();

    // This should trigger compaction since read_pos >= 32
    let _line2 = buf.read_line();

    // After compaction, read_pos should be 0
    assert_eq!(buf.read_pos, 0);
}

#[test]
fn ring_buffer_overflow_protection() {
    let mut buf: RingBuffer<16> = RingBuffer::new();

    // Try to write more than capacity
    let written = buf.push(b"this is a very long string that exceeds the buffer");
    assert_eq!(written, 16);
    assert_eq!(buf.free(), 0);
}

#[test]
fn ring_buffer_crlf_handling() {
    let mut buf: RingBuffer<64> = RingBuffer::new();
    buf.push(b"line1\r\nline2\r\n");

    let line1 = buf.read_line();
    assert!(line1.is_some());
    assert_eq!(line1.unwrap(), b"line1");

    let line2 = buf.read_line();
    assert!(line2.is_some());
    assert_eq!(line2.unwrap(), b"line2");
}

// =============================================================================
// Write Buffer Tests
// =============================================================================

#[test]
fn write_buffer_creation() {
    let buf: WriteBuffer<64> = WriteBuffer::new();
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
}

#[test]
fn write_buffer_write_bytes() {
    let mut buf: WriteBuffer<64> = WriteBuffer::new();
    let written = buf.write(b"hello");
    assert_eq!(written, 5);
    assert_eq!(buf.len(), 5);
    assert_eq!(buf.as_bytes(), b"hello");
}

#[test]
fn write_buffer_write_str() {
    let mut buf: WriteBuffer<64> = WriteBuffer::new();
    buf.write_str("test string");
    assert_eq!(buf.as_bytes(), b"test string");
}

#[test]
fn write_buffer_writeln() {
    let mut buf: WriteBuffer<64> = WriteBuffer::new();
    buf.writeln(b"data");
    assert_eq!(buf.as_bytes(), b"data\r\n");
}

#[test]
fn write_buffer_multiple_writes() {
    let mut buf: WriteBuffer<64> = WriteBuffer::new();
    buf.write(b"hello");
    buf.write(b" ");
    buf.write(b"world");
    assert_eq!(buf.as_bytes(), b"hello world");
}

#[test]
fn write_buffer_clear() {
    let mut buf: WriteBuffer<64> = WriteBuffer::new();
    buf.write(b"some data");
    buf.clear();
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
}

#[test]
fn write_buffer_overflow_protection() {
    let mut buf: WriteBuffer<16> = WriteBuffer::new();
    let written = buf.write(b"this is a very long string");
    assert_eq!(written, 16);
    assert_eq!(buf.len(), 16);
}

#[test]
fn write_buffer_cat_response_format() {
    let mut buf: WriteBuffer<64> = WriteBuffer::new();

    // Simulate CAT frequency response
    let freq = 7074000u32;
    let response = format!("FA{:011};", freq);
    buf.write_str(&response);

    assert_eq!(buf.as_bytes(), b"FA00007074000;");
}

#[test]
fn write_buffer_cat_mode_response() {
    let mut buf: WriteBuffer<64> = WriteBuffer::new();
    buf.write(b"MD2;"); // USB mode
    assert_eq!(buf.as_bytes(), b"MD2;");
}

#[test]
fn write_buffer_cat_id_response() {
    let mut buf: WriteBuffer<64> = WriteBuffer::new();
    buf.write(b"ID019;"); // TS-2000 ID
    assert_eq!(buf.as_bytes(), b"ID019;");
}

// =============================================================================
// Integration-style Tests
// =============================================================================

#[test]
fn cat_command_roundtrip() {
    let mut read_buf: RingBuffer<64> = RingBuffer::new();
    let mut write_buf: WriteBuffer<64> = WriteBuffer::new();

    // Simulate receiving CAT command
    read_buf.push(b"FA;");

    // Parse command
    if let Some(cmd) = read_buf.read_line() {
        if cmd == b"FA" {
            // Generate response
            let freq = 7074000u32;
            let response = format!("FA{:011};", freq);
            write_buf.write_str(&response);
        }
    }

    assert_eq!(write_buf.as_bytes(), b"FA00007074000;");
}

#[test]
fn multiple_cat_commands() {
    let mut read_buf: RingBuffer<128> = RingBuffer::new();

    // Multiple commands in one packet
    read_buf.push(b"FA;MD;ID;");

    let cmd1 = read_buf.read_line();
    let cmd2 = read_buf.read_line();
    let cmd3 = read_buf.read_line();

    assert_eq!(cmd1.unwrap(), b"FA");
    assert_eq!(cmd2.unwrap(), b"MD");
    assert_eq!(cmd3.unwrap(), b"ID");
}

#[test]
fn cat_command_with_data() {
    let mut read_buf: RingBuffer<64> = RingBuffer::new();

    // Set frequency command
    read_buf.push(b"FA00014074000;");

    let cmd = read_buf.read_line();
    assert!(cmd.is_some());

    let cmd_bytes = cmd.unwrap();
    assert_eq!(&cmd_bytes[0..2], b"FA");

    // Parse frequency value
    let freq_str = std::str::from_utf8(&cmd_bytes[2..]).unwrap();
    let freq: u32 = freq_str.parse().unwrap();
    assert_eq!(freq, 14074000);
}
