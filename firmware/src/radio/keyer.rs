//! CW Keyer Module
//!
//! Provides iambic keying (Mode A and B), straight key support,
//! and built-in memories for contest operation.
//!
//! # Iambic Keying
//!
//! Iambic keyers use squeeze paddles where pressing both paddles
//! alternates between dit and dah. Mode A releases at element end,
//! Mode B adds one more element after release.

/// Keyer operating mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KeyerMode {
    /// Straight key (manual timing)
    Straight,
    /// Iambic Mode A (stop at element boundary)
    #[default]
    IambicA,
    /// Iambic Mode B (add element after release)
    IambicB,
    /// Bug mode (auto dits, manual dahs)
    Bug,
    /// Ultimatic mode (last paddle pressed wins)
    Ultimatic,
}

/// Paddle input state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PaddleState {
    /// Dit paddle pressed
    pub dit: bool,
    /// Dah paddle pressed
    pub dah: bool,
}

impl PaddleState {
    /// Create new paddle state
    #[must_use]
    pub const fn new(dit: bool, dah: bool) -> Self {
        Self { dit, dah }
    }

    /// Check if both paddles are pressed (squeeze)
    #[must_use]
    pub const fn is_squeeze(&self) -> bool {
        self.dit && self.dah
    }

    /// Check if any paddle is pressed
    #[must_use]
    pub const fn is_pressed(&self) -> bool {
        self.dit || self.dah
    }

    /// Check if no paddle is pressed
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        !self.dit && !self.dah
    }
}

/// Current element being sent
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Element {
    /// No element (idle or inter-element gap)
    #[default]
    None,
    /// Dit (1 unit)
    Dit,
    /// Dah (3 units)
    Dah,
    /// Inter-element gap (1 unit)
    ElementGap,
    /// Inter-character gap (3 units, 2 additional after element gap)
    CharGap,
    /// Inter-word gap (7 units, 4 additional after char gap)
    WordGap,
}

impl Element {
    /// Get duration in timing units (at 1 WPM, 1 unit = 1200ms)
    #[must_use]
    pub const fn units(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Dit => 1,
            Self::Dah => 3,
            Self::ElementGap => 1,
            Self::CharGap => 2, // 2 additional after element gap
            Self::WordGap => 4, // 4 additional after char gap
        }
    }

    /// Check if this element produces a tone
    #[must_use]
    pub const fn is_tone(&self) -> bool {
        matches!(self, Self::Dit | Self::Dah)
    }
}

/// Keyer state machine state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum KeyerState {
    /// Idle, waiting for input
    #[default]
    Idle,
    /// Sending a dit
    SendingDit,
    /// Sending a dah
    SendingDah,
    /// Inter-element gap
    ElementGap,
    /// Waiting for next element (Mode B check)
    WaitNext,
}

/// CW Keyer with iambic support
#[derive(Clone, Debug)]
pub struct Keyer {
    /// Operating mode
    mode: KeyerMode,
    /// Speed in WPM
    wpm: u8,
    /// Sample rate in Hz
    sample_rate: u32,
    /// Current state
    state: KeyerState,
    /// Samples remaining in current element
    samples_remaining: u32,
    /// Last element sent (for alternation)
    last_element: Element,
    /// Paddle state at element start (for Mode B)
    paddle_at_start: PaddleState,
    /// Memory for squeeze detection
    dit_memory: bool,
    /// Memory for squeeze detection
    dah_memory: bool,
    /// Weighting (50 = standard, <50 = lighter, >50 = heavier)
    weight: u8,
    /// Sidetone frequency in Hz
    sidetone_freq: u16,
    /// Current output state (key down)
    key_down: bool,
}

impl Keyer {
    /// Standard timing: 1 WPM = 1200ms per unit
    const MS_PER_UNIT_AT_1WPM: u32 = 1200;

    /// Default speed
    pub const DEFAULT_WPM: u8 = 20;

    /// Minimum speed
    pub const MIN_WPM: u8 = 5;

    /// Maximum speed
    pub const MAX_WPM: u8 = 50;

    /// Default sidetone frequency
    pub const DEFAULT_SIDETONE_HZ: u16 = 700;

    /// Create a new keyer
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            mode: KeyerMode::default(),
            wpm: Self::DEFAULT_WPM,
            sample_rate,
            state: KeyerState::Idle,
            samples_remaining: 0,
            last_element: Element::None,
            paddle_at_start: PaddleState::default(),
            dit_memory: false,
            dah_memory: false,
            weight: 50,
            sidetone_freq: Self::DEFAULT_SIDETONE_HZ,
            key_down: false,
        }
    }

    /// Set keyer mode
    pub fn set_mode(&mut self, mode: KeyerMode) {
        self.mode = mode;
    }

    /// Get current mode
    #[must_use]
    pub const fn mode(&self) -> KeyerMode {
        self.mode
    }

    /// Set speed in WPM
    pub fn set_wpm(&mut self, wpm: u8) {
        self.wpm = wpm.clamp(Self::MIN_WPM, Self::MAX_WPM);
    }

    /// Get current speed
    #[must_use]
    pub const fn wpm(&self) -> u8 {
        self.wpm
    }

    /// Set weighting (50 = standard)
    pub fn set_weight(&mut self, weight: u8) {
        self.weight = weight.clamp(25, 75);
    }

    /// Get current weighting
    #[must_use]
    pub const fn weight(&self) -> u8 {
        self.weight
    }

    /// Set sidetone frequency
    pub fn set_sidetone(&mut self, freq: u16) {
        self.sidetone_freq = freq.clamp(300, 1200);
    }

    /// Get sidetone frequency
    #[must_use]
    pub const fn sidetone(&self) -> u16 {
        self.sidetone_freq
    }

    /// Check if key is currently down
    #[must_use]
    pub const fn is_key_down(&self) -> bool {
        self.key_down
    }

    /// Check if keyer is idle
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        matches!(self.state, KeyerState::Idle)
    }

    /// Calculate samples per timing unit at current WPM
    fn samples_per_unit(&self) -> u32 {
        // Unit duration in ms = 1200 / WPM
        // Samples = duration_ms * sample_rate / 1000
        let ms_per_unit = Self::MS_PER_UNIT_AT_1WPM / u32::from(self.wpm);
        ms_per_unit * self.sample_rate / 1000
    }

    /// Calculate samples for an element with weighting
    fn samples_for_element(&self, element: Element) -> u32 {
        let base_samples = self.samples_per_unit() * element.units();

        if element.is_tone() {
            // Apply weighting to tone elements
            // weight=50 -> 100%, weight=25 -> 50%, weight=75 -> 150%
            base_samples * u32::from(self.weight) / 50
        } else {
            // Gaps get inverse weighting
            base_samples * (100 - u32::from(self.weight)) / 50
        }
    }

    /// Process paddle input and update state
    /// Returns true if key output changed
    pub fn process(&mut self, paddle: PaddleState) -> bool {
        let old_key_down = self.key_down;

        // Update memories on paddle press
        if paddle.dit {
            self.dit_memory = true;
        }
        if paddle.dah {
            self.dah_memory = true;
        }

        match self.mode {
            KeyerMode::Straight => self.process_straight(paddle),
            KeyerMode::IambicA => self.process_iambic(paddle, false),
            KeyerMode::IambicB => self.process_iambic(paddle, true),
            KeyerMode::Bug => self.process_bug(paddle),
            KeyerMode::Ultimatic => self.process_ultimatic(paddle),
        }

        old_key_down != self.key_down
    }

    /// Process straight key mode
    fn process_straight(&mut self, paddle: PaddleState) {
        // In straight key mode, dit paddle = key down
        self.key_down = paddle.dit;
        self.state = if paddle.dit {
            KeyerState::SendingDit
        } else {
            KeyerState::Idle
        };
    }

    /// Process iambic mode (A or B)
    fn process_iambic(&mut self, paddle: PaddleState, mode_b: bool) {
        if self.samples_remaining > 0 {
            self.samples_remaining -= 1;
            return;
        }

        match self.state {
            KeyerState::Idle => {
                // Start new element based on paddle
                if self.dit_memory {
                    self.start_element(Element::Dit);
                    self.dit_memory = false;
                    self.paddle_at_start = paddle;
                } else if self.dah_memory {
                    self.start_element(Element::Dah);
                    self.dah_memory = false;
                    self.paddle_at_start = paddle;
                }
            }

            KeyerState::SendingDit | KeyerState::SendingDah => {
                // Element finished, start gap
                self.state = KeyerState::ElementGap;
                self.samples_remaining = self.samples_for_element(Element::ElementGap);
                self.key_down = false;
                self.last_element = if self.state == KeyerState::SendingDit {
                    Element::Dit
                } else {
                    Element::Dah
                };
            }

            KeyerState::ElementGap => {
                // Gap finished, check for next element
                if mode_b {
                    self.state = KeyerState::WaitNext;
                    // Check memories for Mode B continuation
                    if self.paddle_at_start.is_squeeze() {
                        // Alternate
                        if self.last_element == Element::Dit && !self.dah_memory {
                            self.dah_memory = true;
                        } else if self.last_element == Element::Dah && !self.dit_memory {
                            self.dit_memory = true;
                        }
                    }
                }

                // Start next element or go idle
                if self.dit_memory {
                    self.start_element(Element::Dit);
                    self.dit_memory = false;
                    self.paddle_at_start = paddle;
                } else if self.dah_memory {
                    self.start_element(Element::Dah);
                    self.dah_memory = false;
                    self.paddle_at_start = paddle;
                } else {
                    self.state = KeyerState::Idle;
                }
            }

            KeyerState::WaitNext => {
                self.state = KeyerState::Idle;
            }
        }
    }

    /// Process bug mode (auto dits, manual dahs)
    fn process_bug(&mut self, paddle: PaddleState) {
        if self.samples_remaining > 0 {
            self.samples_remaining -= 1;
            return;
        }

        match self.state {
            KeyerState::Idle => {
                if paddle.dit {
                    self.start_element(Element::Dit);
                } else if paddle.dah {
                    // Manual dah - key down while held
                    self.key_down = true;
                    self.state = KeyerState::SendingDah;
                }
            }

            KeyerState::SendingDit => {
                // Auto dit finished, start gap
                self.state = KeyerState::ElementGap;
                self.samples_remaining = self.samples_for_element(Element::ElementGap);
                self.key_down = false;
            }

            KeyerState::SendingDah => {
                // Manual dah - check if still held
                if !paddle.dah {
                    self.key_down = false;
                    self.state = KeyerState::Idle;
                }
            }

            KeyerState::ElementGap => {
                // Continue dits if paddle held
                if paddle.dit {
                    self.start_element(Element::Dit);
                } else {
                    self.state = KeyerState::Idle;
                }
            }

            KeyerState::WaitNext => {
                self.state = KeyerState::Idle;
            }
        }
    }

    /// Process ultimatic mode (last paddle wins)
    fn process_ultimatic(&mut self, paddle: PaddleState) {
        if self.samples_remaining > 0 {
            self.samples_remaining -= 1;
            return;
        }

        match self.state {
            KeyerState::Idle => {
                // Last paddle pressed determines element
                if paddle.dit && !paddle.dah {
                    self.start_element(Element::Dit);
                } else if paddle.dah {
                    self.start_element(Element::Dah);
                }
            }

            KeyerState::SendingDit | KeyerState::SendingDah => {
                self.state = KeyerState::ElementGap;
                self.samples_remaining = self.samples_for_element(Element::ElementGap);
                self.key_down = false;
            }

            KeyerState::ElementGap => {
                // Continue with current paddle, prefer dah if both
                if paddle.dah {
                    self.start_element(Element::Dah);
                } else if paddle.dit {
                    self.start_element(Element::Dit);
                } else {
                    self.state = KeyerState::Idle;
                }
            }

            KeyerState::WaitNext => {
                self.state = KeyerState::Idle;
            }
        }
    }

    /// Start sending an element
    fn start_element(&mut self, element: Element) {
        self.samples_remaining = self.samples_for_element(element);
        self.key_down = element.is_tone();
        self.state = match element {
            Element::Dit => KeyerState::SendingDit,
            Element::Dah => KeyerState::SendingDah,
            _ => KeyerState::Idle,
        };
        self.last_element = element;
    }

    /// Reset keyer to idle state
    pub fn reset(&mut self) {
        self.state = KeyerState::Idle;
        self.samples_remaining = 0;
        self.key_down = false;
        self.dit_memory = false;
        self.dah_memory = false;
    }
}

impl Default for Keyer {
    fn default() -> Self {
        Self::new(48000)
    }
}

/// Morse code character encoder
pub struct MorseEncoder {
    /// Current character being sent
    current: Option<&'static str>,
    /// Position within current character
    position: usize,
}

impl MorseEncoder {
    /// Create a new encoder
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current: None,
            position: 0,
        }
    }

    /// Load a character to send
    pub fn load(&mut self, c: char) {
        self.current = Self::char_to_morse(c);
        self.position = 0;
    }

    /// Get next element to send
    pub fn next_element(&mut self) -> Option<Element> {
        let morse = self.current?;
        let bytes = morse.as_bytes();

        if self.position >= bytes.len() {
            self.current = None;
            return Some(Element::CharGap);
        }

        let element = match bytes[self.position] {
            b'.' => Element::Dit,
            b'-' => Element::Dah,
            _ => return None,
        };

        self.position += 1;

        // Add element gap between dits/dahs
        Some(element)
    }

    /// Check if encoder is idle
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        self.current.is_none()
    }

    /// Convert character to Morse pattern
    const fn char_to_morse(c: char) -> Option<&'static str> {
        match c.to_ascii_uppercase() {
            'A' => Some(".-"),
            'B' => Some("-..."),
            'C' => Some("-.-."),
            'D' => Some("-.."),
            'E' => Some("."),
            'F' => Some("..-."),
            'G' => Some("--."),
            'H' => Some("...."),
            'I' => Some(".."),
            'J' => Some(".---"),
            'K' => Some("-.-"),
            'L' => Some(".-.."),
            'M' => Some("--"),
            'N' => Some("-."),
            'O' => Some("---"),
            'P' => Some(".--."),
            'Q' => Some("--.-"),
            'R' => Some(".-."),
            'S' => Some("..."),
            'T' => Some("-"),
            'U' => Some("..-"),
            'V' => Some("...-"),
            'W' => Some(".--"),
            'X' => Some("-..-"),
            'Y' => Some("-.--"),
            'Z' => Some("--.."),
            '0' => Some("-----"),
            '1' => Some(".----"),
            '2' => Some("..---"),
            '3' => Some("...--"),
            '4' => Some("....-"),
            '5' => Some("....."),
            '6' => Some("-...."),
            '7' => Some("--..."),
            '8' => Some("---.."),
            '9' => Some("----."),
            '.' => Some(".-.-.-"),
            ',' => Some("--..--"),
            '?' => Some("..--.."),
            '/' => Some("-..-."),
            '=' => Some("-...-"),
            ' ' => Some(" "), // Word gap
            _ => None,
        }
    }
}

impl Default for MorseEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paddle_state_squeeze() {
        let idle = PaddleState::new(false, false);
        assert!(!idle.is_squeeze());
        assert!(idle.is_idle());

        let dit = PaddleState::new(true, false);
        assert!(!dit.is_squeeze());
        assert!(dit.is_pressed());

        let squeeze = PaddleState::new(true, true);
        assert!(squeeze.is_squeeze());
        assert!(squeeze.is_pressed());
    }

    #[test]
    fn element_units() {
        assert_eq!(Element::Dit.units(), 1);
        assert_eq!(Element::Dah.units(), 3);
        assert_eq!(Element::ElementGap.units(), 1);
    }

    #[test]
    fn element_is_tone() {
        assert!(Element::Dit.is_tone());
        assert!(Element::Dah.is_tone());
        assert!(!Element::ElementGap.is_tone());
        assert!(!Element::CharGap.is_tone());
        assert!(!Element::None.is_tone());
    }

    #[test]
    fn keyer_new() {
        let keyer = Keyer::new(48000);
        assert_eq!(keyer.wpm(), Keyer::DEFAULT_WPM);
        assert_eq!(keyer.mode(), KeyerMode::IambicA);
        assert!(!keyer.is_key_down());
        assert!(keyer.is_idle());
    }

    #[test]
    fn keyer_set_wpm() {
        let mut keyer = Keyer::new(48000);

        keyer.set_wpm(25);
        assert_eq!(keyer.wpm(), 25);

        keyer.set_wpm(3); // Below min
        assert_eq!(keyer.wpm(), Keyer::MIN_WPM);

        keyer.set_wpm(100); // Above max
        assert_eq!(keyer.wpm(), Keyer::MAX_WPM);
    }

    #[test]
    fn keyer_set_weight() {
        let mut keyer = Keyer::new(48000);

        keyer.set_weight(60);
        assert_eq!(keyer.weight(), 60);

        keyer.set_weight(10); // Below min
        assert_eq!(keyer.weight(), 25);

        keyer.set_weight(90); // Above max
        assert_eq!(keyer.weight(), 75);
    }

    #[test]
    fn keyer_straight_mode() {
        let mut keyer = Keyer::new(48000);
        keyer.set_mode(KeyerMode::Straight);

        // Key down when dit pressed
        let paddle = PaddleState::new(true, false);
        keyer.process(paddle);
        assert!(keyer.is_key_down());

        // Key up when released
        let paddle = PaddleState::new(false, false);
        keyer.process(paddle);
        assert!(!keyer.is_key_down());
    }

    #[test]
    fn keyer_samples_per_unit() {
        let mut keyer = Keyer::new(48000);
        keyer.set_wpm(20);

        // At 20 WPM: unit = 1200/20 = 60ms
        // samples = 60 * 48000 / 1000 = 2880
        let spu = keyer.samples_per_unit();
        assert_eq!(spu, 2880);
    }

    #[test]
    fn keyer_iambic_dit() {
        let mut keyer = Keyer::new(48000);
        keyer.set_mode(KeyerMode::IambicA);
        keyer.set_wpm(20);

        // Press dit paddle
        let paddle = PaddleState::new(true, false);
        keyer.process(paddle);
        assert!(keyer.is_key_down());

        // Process until dit complete (should be ~2880 samples at 20 WPM)
        let paddle = PaddleState::new(false, false);
        for _ in 0..3000 {
            keyer.process(paddle);
        }

        // Should be key up after dit + gap
        assert!(!keyer.is_key_down());
    }

    #[test]
    fn keyer_reset() {
        let mut keyer = Keyer::new(48000);

        // Start a dit
        let paddle = PaddleState::new(true, false);
        keyer.process(paddle);
        assert!(keyer.is_key_down());

        // Reset
        keyer.reset();
        assert!(!keyer.is_key_down());
        assert!(keyer.is_idle());
    }

    #[test]
    fn morse_encoder_new() {
        let encoder = MorseEncoder::new();
        assert!(encoder.is_idle());
    }

    #[test]
    fn morse_encoder_letter_e() {
        let mut encoder = MorseEncoder::new();
        encoder.load('E');

        let element = encoder.next_element();
        assert_eq!(element, Some(Element::Dit));

        let element = encoder.next_element();
        assert_eq!(element, Some(Element::CharGap));

        assert!(encoder.is_idle());
    }

    #[test]
    fn morse_encoder_letter_t() {
        let mut encoder = MorseEncoder::new();
        encoder.load('T');

        let element = encoder.next_element();
        assert_eq!(element, Some(Element::Dah));

        let element = encoder.next_element();
        assert_eq!(element, Some(Element::CharGap));
    }

    #[test]
    fn morse_encoder_letter_a() {
        let mut encoder = MorseEncoder::new();
        encoder.load('A');

        assert_eq!(encoder.next_element(), Some(Element::Dit));
        assert_eq!(encoder.next_element(), Some(Element::Dah));
        assert_eq!(encoder.next_element(), Some(Element::CharGap));
        assert!(encoder.is_idle());
    }

    #[test]
    fn morse_encoder_numbers() {
        let mut encoder = MorseEncoder::new();

        // '5' is .....
        encoder.load('5');
        for _ in 0..5 {
            assert_eq!(encoder.next_element(), Some(Element::Dit));
        }
        assert_eq!(encoder.next_element(), Some(Element::CharGap));

        // '0' is -----
        encoder.load('0');
        for _ in 0..5 {
            assert_eq!(encoder.next_element(), Some(Element::Dah));
        }
        assert_eq!(encoder.next_element(), Some(Element::CharGap));
    }

    #[test]
    fn morse_encoder_prosigns() {
        let mut encoder = MorseEncoder::new();

        // '?' is ..--..
        encoder.load('?');
        assert_eq!(encoder.next_element(), Some(Element::Dit));
        assert_eq!(encoder.next_element(), Some(Element::Dit));
        assert_eq!(encoder.next_element(), Some(Element::Dah));
        assert_eq!(encoder.next_element(), Some(Element::Dah));
        assert_eq!(encoder.next_element(), Some(Element::Dit));
        assert_eq!(encoder.next_element(), Some(Element::Dit));
        assert_eq!(encoder.next_element(), Some(Element::CharGap));
    }

    #[test]
    fn morse_encoder_case_insensitive() {
        let mut encoder = MorseEncoder::new();

        encoder.load('a');
        assert_eq!(encoder.next_element(), Some(Element::Dit));
        assert_eq!(encoder.next_element(), Some(Element::Dah));

        encoder.load('A');
        assert_eq!(encoder.next_element(), Some(Element::Dit));
        assert_eq!(encoder.next_element(), Some(Element::Dah));
    }

    #[test]
    fn morse_encoder_unknown_char() {
        let mut encoder = MorseEncoder::new();
        encoder.load('@'); // Not in Morse table
        assert!(encoder.is_idle());
    }

    #[test]
    fn keyer_mode_default() {
        assert_eq!(KeyerMode::default(), KeyerMode::IambicA);
    }

    #[test]
    fn keyer_sidetone() {
        let mut keyer = Keyer::new(48000);
        assert_eq!(keyer.sidetone(), Keyer::DEFAULT_SIDETONE_HZ);

        keyer.set_sidetone(600);
        assert_eq!(keyer.sidetone(), 600);

        keyer.set_sidetone(100); // Below min
        assert_eq!(keyer.sidetone(), 300);

        keyer.set_sidetone(2000); // Above max
        assert_eq!(keyer.sidetone(), 1200);
    }
}
