//! Communication Protocols
//!
//! CAT (Computer Aided Transceiver) command parsing and handling.
//! Implements Kenwood-style TS-2000 compatible commands.

use heapless::{String, Vec};

#[cfg(feature = "embedded")]
use crate::radio::state::RadioEvent;
use crate::types::{Frequency, Mode, PowerLevel};

/// Maximum command length
pub const MAX_CMD_LEN: usize = 64;

/// CAT command parser
pub struct CatParser {
    /// Command buffer
    buffer: Vec<u8, MAX_CMD_LEN>,
}

impl CatParser {
    /// Create a new CAT parser
    #[must_use]
    pub const fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Feed a byte to the parser
    /// Returns a command if one is complete
    pub fn feed(&mut self, byte: u8) -> Option<CatCommand> {
        // Commands end with ';'
        if byte == b';' {
            let cmd = self.parse_buffer();
            self.buffer.clear();
            cmd
        } else if byte == b'\r' || byte == b'\n' {
            // Ignore line endings
            None
        } else {
            // Add to buffer
            let _ = self.buffer.push(byte);

            // Prevent overflow
            if self.buffer.len() >= MAX_CMD_LEN {
                self.buffer.clear();
            }

            None
        }
    }

    /// Parse the current buffer as a command
    fn parse_buffer(&self) -> Option<CatCommand> {
        if self.buffer.len() < 2 {
            return None;
        }

        let cmd = core::str::from_utf8(&self.buffer).ok()?;

        // Parse based on first two characters (Kenwood style)
        match &cmd[..2] {
            "FA" => self.parse_frequency(cmd, false),
            "FB" => self.parse_frequency(cmd, true),
            "MD" => self.parse_mode(cmd),
            "IF" => Some(CatCommand::ReadStatus),
            "ID" => Some(CatCommand::ReadId),
            "PS" => self.parse_power_switch(cmd),
            "TX" => Some(CatCommand::Transmit(true)),
            "RX" => Some(CatCommand::Transmit(false)),
            "AG" => self.parse_af_gain(cmd),
            "PC" => self.parse_power(cmd),
            "SH" => self.parse_filter(cmd),
            "SL" => self.parse_filter(cmd),
            "AI" => self.parse_auto_info(cmd),
            "FR" => self.parse_vfo_select(cmd, true),
            "FT" => self.parse_vfo_select(cmd, false),
            "VX" => self.parse_vox(cmd),
            "GT" => self.parse_agc(cmd),
            "NB" => self.parse_nb(cmd),
            "PA" => self.parse_preamp(cmd),
            "RA" => self.parse_att(cmd),
            "UP" => Some(CatCommand::TuneUp),
            "DN" => Some(CatCommand::TuneDown),
            _ => Some(CatCommand::Unknown(cmd.chars().take(2).collect())),
        }
    }

    fn parse_frequency(&self, cmd: &str, vfo_b: bool) -> Option<CatCommand> {
        if cmd.len() == 2 {
            // Query
            Some(CatCommand::ReadFrequency(vfo_b))
        } else if cmd.len() >= 13 {
            // Set: FAnnnnnnnnnn; (11 digits)
            let freq_str = &cmd[2..13];
            let hz: u32 = freq_str.parse().ok()?;
            let freq = Frequency::from_hz(hz)?;
            Some(CatCommand::SetFrequency(freq, vfo_b))
        } else {
            None
        }
    }

    fn parse_mode(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() == 2 {
            Some(CatCommand::ReadMode)
        } else if cmd.len() >= 3 {
            let mode_char = cmd.chars().nth(2)?;
            let mode = match mode_char {
                '1' => Mode::Lsb,
                '2' => Mode::Usb,
                '3' => Mode::Cw,
                '4' => Mode::Fm,
                '5' => Mode::Am,
                '7' => Mode::CwR,
                _ => return None,
            };
            Some(CatCommand::SetMode(mode))
        } else {
            None
        }
    }

    fn parse_power_switch(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() == 2 {
            Some(CatCommand::ReadPowerSwitch)
        } else if cmd.len() >= 3 {
            let on = cmd.chars().nth(2)? == '1';
            Some(CatCommand::SetPowerSwitch(on))
        } else {
            None
        }
    }

    fn parse_af_gain(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() == 2 || cmd.len() == 3 {
            Some(CatCommand::ReadAfGain)
        } else if cmd.len() >= 6 {
            let gain: u8 = cmd[3..6].parse().ok()?;
            Some(CatCommand::SetAfGain(gain))
        } else {
            None
        }
    }

    fn parse_power(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() == 2 {
            Some(CatCommand::ReadPower)
        } else if cmd.len() >= 5 {
            let pwr: u8 = cmd[2..5].parse().ok()?;
            Some(CatCommand::SetPower(PowerLevel::from_percent(pwr)))
        } else {
            None
        }
    }

    fn parse_filter(&self, _cmd: &str) -> Option<CatCommand> {
        // Filter width commands
        Some(CatCommand::ReadFilter)
    }

    fn parse_auto_info(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() >= 3 {
            let on = cmd.chars().nth(2)? == '1';
            Some(CatCommand::SetAutoInfo(on))
        } else {
            Some(CatCommand::ReadAutoInfo)
        }
    }

    fn parse_vfo_select(&self, cmd: &str, rx: bool) -> Option<CatCommand> {
        if cmd.len() >= 3 {
            let vfo = cmd.chars().nth(2)? == '1';
            if rx {
                Some(CatCommand::SetRxVfo(vfo))
            } else {
                Some(CatCommand::SetTxVfo(vfo))
            }
        } else if rx {
            Some(CatCommand::ReadRxVfo)
        } else {
            Some(CatCommand::ReadTxVfo)
        }
    }

    fn parse_vox(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() >= 3 {
            let on = cmd.chars().nth(2)? == '1';
            Some(CatCommand::SetVox(on))
        } else {
            Some(CatCommand::ReadVox)
        }
    }

    fn parse_agc(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() >= 5 {
            let agc: u8 = cmd[2..5].parse().ok()?;
            Some(CatCommand::SetAgc(agc))
        } else {
            Some(CatCommand::ReadAgc)
        }
    }

    fn parse_nb(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() >= 3 {
            let on = cmd.chars().nth(2)? == '1';
            Some(CatCommand::SetNb(on))
        } else {
            Some(CatCommand::ReadNb)
        }
    }

    fn parse_preamp(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() >= 3 {
            let on = cmd.chars().nth(2)? == '1';
            Some(CatCommand::SetPreamp(on))
        } else {
            Some(CatCommand::ReadPreamp)
        }
    }

    fn parse_att(&self, cmd: &str) -> Option<CatCommand> {
        if cmd.len() >= 4 {
            let on = cmd[2..4].parse::<u8>().ok()? > 0;
            Some(CatCommand::SetAtt(on))
        } else {
            Some(CatCommand::ReadAtt)
        }
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Default for CatParser {
    fn default() -> Self {
        Self::new()
    }
}

/// CAT command parsed from serial input
#[derive(Clone, Debug)]
pub enum CatCommand {
    /// Read VFO frequency (VFO B if true)
    ReadFrequency(bool),
    /// Set VFO frequency (frequency, VFO B if true)
    SetFrequency(Frequency, bool),
    /// Read operating mode
    ReadMode,
    /// Set operating mode
    SetMode(Mode),
    /// Read transceiver status (IF command)
    ReadStatus,
    /// Read transceiver ID
    ReadId,
    /// Read power switch state
    ReadPowerSwitch,
    /// Set power switch state
    SetPowerSwitch(bool),
    /// Transmit on/off
    Transmit(bool),
    /// Read AF gain level
    ReadAfGain,
    /// Set AF gain level
    SetAfGain(u8),
    /// Read TX power level
    ReadPower,
    /// Set TX power level
    SetPower(PowerLevel),
    /// Read filter setting
    ReadFilter,
    /// Read auto-info state
    ReadAutoInfo,
    /// Set auto-info state
    SetAutoInfo(bool),
    /// Read RX VFO selection
    ReadRxVfo,
    /// Set RX VFO selection (VFO B if true)
    SetRxVfo(bool),
    /// Read TX VFO selection
    ReadTxVfo,
    /// Set TX VFO selection (VFO B if true)
    SetTxVfo(bool),
    /// Read VOX state
    ReadVox,
    /// Set VOX state
    SetVox(bool),
    /// Read AGC setting
    ReadAgc,
    /// Set AGC setting
    SetAgc(u8),
    /// Read noise blanker state
    ReadNb,
    /// Set noise blanker state
    SetNb(bool),
    /// Read preamp state
    ReadPreamp,
    /// Set preamp state
    SetPreamp(bool),
    /// Read attenuator state
    ReadAtt,
    /// Set attenuator state
    SetAtt(bool),
    /// Tune up one step
    TuneUp,
    /// Tune down one step
    TuneDown,
    /// Unknown/unparsed command
    Unknown(String<4>),
}

#[cfg(feature = "embedded")]
impl defmt::Format for CatCommand {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::ReadFrequency(b) => defmt::write!(f, "ReadFreq({})", b),
            Self::SetFrequency(freq, b) => defmt::write!(f, "SetFreq({}, {})", freq, b),
            Self::ReadMode => defmt::write!(f, "ReadMode"),
            Self::SetMode(m) => defmt::write!(f, "SetMode({})", m),
            Self::ReadStatus => defmt::write!(f, "ReadStatus"),
            Self::ReadId => defmt::write!(f, "ReadId"),
            Self::Transmit(tx) => defmt::write!(f, "TX({})", tx),
            _ => defmt::write!(f, "CAT(...)"),
        }
    }
}

/// Convert CAT command to radio event
#[cfg(feature = "embedded")]
impl CatCommand {
    /// Convert to radio event if applicable
    #[must_use]
    pub fn to_radio_event(&self) -> Option<RadioEvent> {
        match self {
            Self::SetFrequency(freq, false) => Some(RadioEvent::SetFrequency(*freq)),
            Self::SetMode(mode) => Some(RadioEvent::SetMode(*mode)),
            Self::SetPower(power) => Some(RadioEvent::SetPower(*power)),
            Self::Transmit(true) => Some(RadioEvent::StartTx),
            Self::Transmit(false) => Some(RadioEvent::StopTx),
            Self::SetNb(on) => {
                if *on {
                    Some(RadioEvent::ToggleNb)
                } else {
                    None // Would need current state
                }
            }
            Self::SetPreamp(on) => {
                if *on {
                    Some(RadioEvent::TogglePreamp)
                } else {
                    None
                }
            }
            Self::SetAtt(on) => {
                if *on {
                    Some(RadioEvent::ToggleAtt)
                } else {
                    None
                }
            }
            Self::TuneUp => Some(RadioEvent::Tune(1)),
            Self::TuneDown => Some(RadioEvent::Tune(-1)),
            _ => None,
        }
    }
}

/// CAT response formatter
pub struct CatResponse {
    buffer: String<MAX_CMD_LEN>,
}

impl CatResponse {
    /// Create a new response formatter
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Format frequency response
    pub fn frequency(&mut self, freq: Frequency, vfo_b: bool) {
        self.buffer.clear();
        let prefix = if vfo_b { "FB" } else { "FA" };
        let hz = freq.as_hz();
        // Format as 11-digit number
        let _ = core::fmt::write(
            &mut self.buffer,
            format_args!("{prefix}{hz:011};"),
        );
    }

    /// Format mode response
    pub fn mode(&mut self, mode: Mode) {
        self.buffer.clear();
        let code = match mode {
            Mode::Lsb => '1',
            Mode::Usb => '2',
            Mode::Cw => '3',
            Mode::Fm => '4',
            Mode::Am => '5',
            Mode::CwR => '7',
        };
        let _ = core::fmt::write(&mut self.buffer, format_args!("MD{code};"));
    }

    /// Format ID response (TS-2000 compatible)
    pub fn id(&mut self) {
        self.buffer.clear();
        let _ = self.buffer.push_str("ID019;");
    }

    /// Format power response
    pub fn power(&mut self, power: PowerLevel) {
        self.buffer.clear();
        let _ = core::fmt::write(
            &mut self.buffer,
            format_args!("PC{:03};", power.as_percent()),
        );
    }

    /// Format status response (IF command)
    pub fn status(&mut self, freq: Frequency, mode: Mode, tx: bool) {
        self.buffer.clear();
        let mode_code = match mode {
            Mode::Lsb => '1',
            Mode::Usb => '2',
            Mode::Cw => '3',
            Mode::Fm => '4',
            Mode::Am => '5',
            Mode::CwR => '7',
        };
        let tx_code = if tx { '1' } else { '0' };

        // IF response: IFaaaaaaaaaaaoooooccccctb...;
        // a = frequency (11 digits)
        // o = offset (5 digits)
        // c = RIT/XIT offset (5 digits)
        // t = RIT on
        // b = XIT on
        let _ = core::fmt::write(
            &mut self.buffer,
            format_args!(
                "IF{:011}00000+0000000000{}0000000000{};",
                freq.as_hz(),
                mode_code,
                tx_code
            ),
        );
    }

    /// Get the response string
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Get the response bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.buffer.as_bytes()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Default for CatResponse {
    fn default() -> Self {
        Self::new()
    }
}
