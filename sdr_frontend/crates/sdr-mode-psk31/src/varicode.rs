//! Varicode encoding for PSK31.
//!
//! Varicode is a variable-length encoding where common characters
//! have shorter codes. Characters are delimited by "00" bit sequences.


/// Varicode lookup table entry.
#[derive(Clone, Copy, Debug)]
pub struct VaricodeEntry {
    /// Character
    pub ch: char,
    /// Varicode bits (LSB first)
    pub code: u16,
    /// Number of bits in code
    pub bits: u8,
}

/// Varicode table (PSK31 standard).
///
/// Characters are ordered by ASCII value for fast lookup during encoding.
pub static VARICODE_TABLE: &[VaricodeEntry] = &[
    VaricodeEntry { ch: '\0', code: 0b1010101011, bits: 10 },  // NUL
    VaricodeEntry { ch: '\n', code: 0b11101, bits: 5 },        // LF
    VaricodeEntry { ch: '\r', code: 0b11111, bits: 5 },        // CR
    VaricodeEntry { ch: ' ', code: 0b1, bits: 1 },             // Space
    VaricodeEntry { ch: '!', code: 0b111111111, bits: 9 },
    VaricodeEntry { ch: '"', code: 0b101011111, bits: 9 },
    VaricodeEntry { ch: '#', code: 0b111110101, bits: 9 },
    VaricodeEntry { ch: '$', code: 0b111011011, bits: 9 },
    VaricodeEntry { ch: '%', code: 0b1011010101, bits: 10 },
    VaricodeEntry { ch: '&', code: 0b1010111011, bits: 10 },
    VaricodeEntry { ch: '\'', code: 0b101111111, bits: 9 },
    VaricodeEntry { ch: '(', code: 0b11111011, bits: 8 },
    VaricodeEntry { ch: ')', code: 0b11110111, bits: 8 },
    VaricodeEntry { ch: '*', code: 0b101101111, bits: 9 },
    VaricodeEntry { ch: '+', code: 0b111011111, bits: 9 },
    VaricodeEntry { ch: ',', code: 0b1110101, bits: 7 },
    VaricodeEntry { ch: '-', code: 0b110101, bits: 6 },
    VaricodeEntry { ch: '.', code: 0b1010111, bits: 7 },
    VaricodeEntry { ch: '/', code: 0b110101111, bits: 9 },
    VaricodeEntry { ch: '0', code: 0b10110111, bits: 8 },
    VaricodeEntry { ch: '1', code: 0b10111101, bits: 8 },
    VaricodeEntry { ch: '2', code: 0b11101101, bits: 8 },
    VaricodeEntry { ch: '3', code: 0b10101101, bits: 8 },
    VaricodeEntry { ch: '4', code: 0b10110101, bits: 8 },
    VaricodeEntry { ch: '5', code: 0b11011011, bits: 8 },
    VaricodeEntry { ch: '6', code: 0b11010101, bits: 8 },
    VaricodeEntry { ch: '7', code: 0b110101101, bits: 9 },
    VaricodeEntry { ch: '8', code: 0b110101011, bits: 9 },
    VaricodeEntry { ch: '9', code: 0b110110111, bits: 9 },
    VaricodeEntry { ch: ':', code: 0b11110101, bits: 8 },
    VaricodeEntry { ch: ';', code: 0b110111101, bits: 9 },
    VaricodeEntry { ch: '<', code: 0b111101101, bits: 9 },
    VaricodeEntry { ch: '=', code: 0b1010101, bits: 7 },
    VaricodeEntry { ch: '>', code: 0b111010111, bits: 9 },
    VaricodeEntry { ch: '?', code: 0b1010101111, bits: 10 },
    VaricodeEntry { ch: '@', code: 0b1010111101, bits: 10 },
    VaricodeEntry { ch: 'A', code: 0b1111101, bits: 7 },
    VaricodeEntry { ch: 'B', code: 0b11101011, bits: 8 },
    VaricodeEntry { ch: 'C', code: 0b10101101, bits: 8 },
    VaricodeEntry { ch: 'D', code: 0b10110101, bits: 8 },
    VaricodeEntry { ch: 'E', code: 0b1110111, bits: 7 },
    VaricodeEntry { ch: 'F', code: 0b11011011, bits: 8 },
    VaricodeEntry { ch: 'G', code: 0b11111101, bits: 8 },
    VaricodeEntry { ch: 'H', code: 0b101010101, bits: 9 },
    VaricodeEntry { ch: 'I', code: 0b1111111, bits: 7 },
    VaricodeEntry { ch: 'J', code: 0b111111101, bits: 9 },
    VaricodeEntry { ch: 'K', code: 0b101111101, bits: 9 },
    VaricodeEntry { ch: 'L', code: 0b11010111, bits: 8 },
    VaricodeEntry { ch: 'M', code: 0b10111011, bits: 8 },
    VaricodeEntry { ch: 'N', code: 0b11011101, bits: 8 },
    VaricodeEntry { ch: 'O', code: 0b10101011, bits: 8 },
    VaricodeEntry { ch: 'P', code: 0b11010101, bits: 8 },
    VaricodeEntry { ch: 'Q', code: 0b111011101, bits: 9 },
    VaricodeEntry { ch: 'R', code: 0b10101111, bits: 8 },
    VaricodeEntry { ch: 'S', code: 0b1101111, bits: 7 },
    VaricodeEntry { ch: 'T', code: 0b1101101, bits: 7 },
    VaricodeEntry { ch: 'U', code: 0b101010111, bits: 9 },
    VaricodeEntry { ch: 'V', code: 0b110110101, bits: 9 },
    VaricodeEntry { ch: 'W', code: 0b101011101, bits: 9 },
    VaricodeEntry { ch: 'X', code: 0b101110101, bits: 9 },
    VaricodeEntry { ch: 'Y', code: 0b101111011, bits: 9 },
    VaricodeEntry { ch: 'Z', code: 0b1010101101, bits: 10 },
    VaricodeEntry { ch: '[', code: 0b111110111, bits: 9 },
    VaricodeEntry { ch: '\\', code: 0b111101111, bits: 9 },
    VaricodeEntry { ch: ']', code: 0b111111011, bits: 9 },
    VaricodeEntry { ch: '^', code: 0b1010111111, bits: 10 },
    VaricodeEntry { ch: '_', code: 0b101101101, bits: 9 },
    VaricodeEntry { ch: '`', code: 0b1011011111, bits: 10 },
    VaricodeEntry { ch: 'a', code: 0b1011, bits: 4 },
    VaricodeEntry { ch: 'b', code: 0b1011111, bits: 7 },
    VaricodeEntry { ch: 'c', code: 0b101111, bits: 6 },
    VaricodeEntry { ch: 'd', code: 0b101101, bits: 6 },
    VaricodeEntry { ch: 'e', code: 0b11, bits: 2 },
    VaricodeEntry { ch: 'f', code: 0b111101, bits: 6 },
    VaricodeEntry { ch: 'g', code: 0b1011011, bits: 7 },
    VaricodeEntry { ch: 'h', code: 0b101011, bits: 6 },
    VaricodeEntry { ch: 'i', code: 0b1101, bits: 4 },
    VaricodeEntry { ch: 'j', code: 0b111101011, bits: 9 },
    VaricodeEntry { ch: 'k', code: 0b10111111, bits: 8 },
    VaricodeEntry { ch: 'l', code: 0b11011, bits: 5 },
    VaricodeEntry { ch: 'm', code: 0b111011, bits: 6 },
    VaricodeEntry { ch: 'n', code: 0b1111, bits: 4 },
    VaricodeEntry { ch: 'o', code: 0b111, bits: 3 },
    VaricodeEntry { ch: 'p', code: 0b111111, bits: 6 },
    VaricodeEntry { ch: 'q', code: 0b110111111, bits: 9 },
    VaricodeEntry { ch: 'r', code: 0b10101, bits: 5 },
    VaricodeEntry { ch: 's', code: 0b10111, bits: 5 },
    VaricodeEntry { ch: 't', code: 0b101, bits: 3 },
    VaricodeEntry { ch: 'u', code: 0b110111, bits: 6 },
    VaricodeEntry { ch: 'v', code: 0b1111011, bits: 7 },
    VaricodeEntry { ch: 'w', code: 0b1101011, bits: 7 },
    VaricodeEntry { ch: 'x', code: 0b11011111, bits: 8 },
    VaricodeEntry { ch: 'y', code: 0b1011101, bits: 7 },
    VaricodeEntry { ch: 'z', code: 0b111010101, bits: 9 },
    VaricodeEntry { ch: '{', code: 0b1010110111, bits: 10 },
    VaricodeEntry { ch: '|', code: 0b110111011, bits: 9 },
    VaricodeEntry { ch: '}', code: 0b1010110101, bits: 10 },
    VaricodeEntry { ch: '~', code: 0b1011010111, bits: 10 },
];

/// Find varicode entry for a character.
pub fn lookup_char(ch: char) -> Option<&'static VaricodeEntry> {
    VARICODE_TABLE.iter().find(|e| e.ch == ch)
}

/// Decode varicode value to character.
pub fn decode_varicode(code: u16) -> Option<char> {
    VARICODE_TABLE.iter().find(|e| e.code == code).map(|e| e.ch)
}

/// Varicode decoder - accumulates bits until character is decoded.
#[derive(Clone, Debug, Default)]
pub struct VaricodeDecoder {
    /// Bit accumulator
    accumulator: u16,
    /// Number of bits accumulated
    bit_count: u8,
    /// Consecutive zeros count (for delimiter detection)
    zero_count: u8,
}

/// Varicode decode error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaricodeError {
    /// Bit pattern overflow (too many bits without delimiter)
    Overflow,
    /// Invalid varicode pattern
    InvalidCode,
}

impl VaricodeDecoder {
    /// Create a new varicode decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            accumulator: 0,
            bit_count: 0,
            zero_count: 0,
        }
    }

    /// Push a single bit and attempt to decode.
    ///
    /// Returns:
    /// - `Ok(Some(char))` when a character is decoded
    /// - `Ok(None)` when more bits are needed
    /// - `Err(VaricodeError)` on decode error
    pub fn push_bit(&mut self, bit: bool) -> Result<Option<char>, VaricodeError> {
        if bit {
            // If we had pending zeros, shift them in first
            for _ in 0..self.zero_count {
                self.accumulator <<= 1;
                self.bit_count += 1;
            }
            self.zero_count = 0;

            // Shift in a 1 bit
            self.accumulator = (self.accumulator << 1) | 1;
            self.bit_count += 1;
        } else {
            self.zero_count += 1;

            // Two consecutive zeros = character delimiter
            if self.zero_count >= 2 && self.bit_count > 0 {
                let code = self.accumulator;
                let result = decode_varicode(code);
                self.reset();

                return match result {
                    Some(ch) => Ok(Some(ch)),
                    None => Err(VaricodeError::InvalidCode),
                };
            }
        }

        // Overflow protection (max varicode is 10 bits + 2 zeros)
        if self.bit_count > 12 {
            self.reset();
            return Err(VaricodeError::Overflow);
        }

        Ok(None)
    }

    /// Reset decoder state.
    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.bit_count = 0;
        self.zero_count = 0;
    }
}

/// Varicode encoder - converts characters to bit stream.
#[derive(Clone, Debug, Default)]
pub struct VaricodeEncoder {
    /// Current code being transmitted
    current_code: u16,
    /// Number of code bits (not including delimiter)
    code_bits: u8,
    /// Bits remaining to output (code + delimiter)
    bits_remaining: u8,
    /// Character queue
    queue: heapless::Deque<char, 64>,
}

impl VaricodeEncoder {
    /// Create a new varicode encoder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current_code: 0,
            code_bits: 0,
            bits_remaining: 0,
            queue: heapless::Deque::new(),
        }
    }

    /// Queue a character for encoding.
    pub fn queue_char(&mut self, ch: char) -> bool {
        self.queue.push_back(ch).is_ok()
    }

    /// Queue a string for encoding.
    pub fn queue_string(&mut self, s: &str) {
        for ch in s.chars() {
            let _ = self.queue_char(ch);
        }
    }

    /// Get next bit to transmit.
    ///
    /// Returns `None` when queue is empty and current code is done.
    /// Bits are transmitted MSB-first, followed by "00" delimiter.
    pub fn next_bit(&mut self) -> Option<bool> {
        // If we're in the middle of a code, continue
        if self.bits_remaining > 0 {
            self.bits_remaining -= 1;

            if self.bits_remaining >= 2 {
                // Output code bits MSB-first
                // bits_remaining is now (code_bits + 1) down to 2
                // We want to output from position (code_bits-1) down to 0
                let code_bit_pos = self.bits_remaining - 2;
                let bit = (self.current_code >> code_bit_pos) & 1 != 0;
                return Some(bit);
            } else {
                // Output delimiter zeros
                return Some(false);
            }
        }

        // Need to load next character
        let ch = self.queue.pop_front()?;

        if let Some(entry) = lookup_char(ch) {
            // Load the code (with trailing zeros for delimiter)
            self.current_code = entry.code;
            self.code_bits = entry.bits;
            self.bits_remaining = entry.bits + 2; // +2 for "00" delimiter
            self.next_bit()
        } else {
            // Unknown character, skip
            self.next_bit()
        }
    }

    /// Check if encoder is idle (nothing to transmit).
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.bits_remaining == 0 && self.queue.is_empty()
    }

    /// Clear the queue.
    pub fn clear(&mut self) {
        self.queue.clear();
        self.current_code = 0;
        self.code_bits = 0;
        self.bits_remaining = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::String;

    #[test]
    fn test_lookup_common_chars() {
        assert!(lookup_char('e').is_some());
        assert!(lookup_char(' ').is_some());
        assert!(lookup_char('E').is_some());
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut encoder = VaricodeEncoder::new();
        let mut decoder = VaricodeDecoder::new();

        encoder.queue_char('H');
        encoder.queue_char('i');

        let mut decoded = String::<8>::new();

        while let Some(bit) = encoder.next_bit() {
            if let Ok(Some(ch)) = decoder.push_bit(bit) {
                let _ = decoded.push(ch);
            }
        }

        assert_eq!(decoded.as_str(), "Hi");
    }

    #[test]
    fn test_space_is_shortest() {
        let space = lookup_char(' ').unwrap();
        assert_eq!(space.bits, 1);
    }

    #[test]
    fn test_e_is_short() {
        let e = lookup_char('e').unwrap();
        assert_eq!(e.bits, 2);
    }
}
