//! CAT Protocol Parser Tests
//!
//! Tests for Kenwood TS-2000 compatible CAT command parsing.

use sdr_firmware::protocol::{CatCommand, CatParser, CatResponse};
use sdr_firmware::types::{Frequency, Mode, PowerLevel};

// ============================================================================
// Parser Basic Tests
// ============================================================================

#[test]
fn test_parser_creation() {
    let _parser = CatParser::new();
}

#[test]
fn test_parser_default() {
    let _parser = CatParser::default();
}

#[test]
fn test_parser_clear() {
    let mut parser = CatParser::new();
    parser.feed(b'F');
    parser.feed(b'A');
    parser.clear();
    // After clear, should need full command again
    assert!(parser.feed(b';').is_none());
}

// ============================================================================
// Frequency Command Tests
// ============================================================================

#[test]
fn test_parse_read_frequency_vfo_a() {
    let mut parser = CatParser::new();
    parser.feed(b'F');
    parser.feed(b'A');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadFrequency(false))));
}

#[test]
fn test_parse_read_frequency_vfo_b() {
    let mut parser = CatParser::new();
    parser.feed(b'F');
    parser.feed(b'B');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadFrequency(true))));
}

#[test]
fn test_parse_set_frequency_vfo_a() {
    let mut parser = CatParser::new();
    // FA00007074000; = 7.074 MHz
    for c in b"FA00007074000" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    match cmd {
        Some(CatCommand::SetFrequency(freq, false)) => {
            assert_eq!(freq.as_hz(), 7_074_000);
        }
        _ => panic!("Expected SetFrequency command"),
    }
}

#[test]
fn test_parse_set_frequency_vfo_b() {
    let mut parser = CatParser::new();
    // FB00014070000; = 14.070 MHz
    for c in b"FB00014070000" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    match cmd {
        Some(CatCommand::SetFrequency(freq, true)) => {
            assert_eq!(freq.as_hz(), 14_070_000);
        }
        _ => panic!("Expected SetFrequency command for VFO B"),
    }
}

#[test]
fn test_parse_frequency_80m_band() {
    let mut parser = CatParser::new();
    for c in b"FA00003573000" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    match cmd {
        Some(CatCommand::SetFrequency(freq, _)) => {
            assert_eq!(freq.as_hz(), 3_573_000);
        }
        _ => panic!("Expected SetFrequency command"),
    }
}

#[test]
fn test_parse_frequency_invalid_too_low() {
    let mut parser = CatParser::new();
    // Below minimum (100 kHz)
    for c in b"FA00000050000" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    // Should return None for invalid frequency
    assert!(cmd.is_none());
}

// ============================================================================
// Mode Command Tests
// ============================================================================

#[test]
fn test_parse_read_mode() {
    let mut parser = CatParser::new();
    parser.feed(b'M');
    parser.feed(b'D');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadMode)));
}

#[test]
fn test_parse_set_mode_lsb() {
    let mut parser = CatParser::new();
    for c in b"MD1" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetMode(Mode::Lsb))));
}

#[test]
fn test_parse_set_mode_usb() {
    let mut parser = CatParser::new();
    for c in b"MD2" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetMode(Mode::Usb))));
}

#[test]
fn test_parse_set_mode_cw() {
    let mut parser = CatParser::new();
    for c in b"MD3" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetMode(Mode::Cw))));
}

#[test]
fn test_parse_set_mode_fm() {
    let mut parser = CatParser::new();
    for c in b"MD4" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetMode(Mode::Fm))));
}

#[test]
fn test_parse_set_mode_am() {
    let mut parser = CatParser::new();
    for c in b"MD5" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetMode(Mode::Am))));
}

#[test]
fn test_parse_set_mode_cw_reverse() {
    let mut parser = CatParser::new();
    for c in b"MD7" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetMode(Mode::CwR))));
}

// ============================================================================
// Status and ID Commands
// ============================================================================

#[test]
fn test_parse_read_status() {
    let mut parser = CatParser::new();
    parser.feed(b'I');
    parser.feed(b'F');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadStatus)));
}

#[test]
fn test_parse_read_id() {
    let mut parser = CatParser::new();
    parser.feed(b'I');
    parser.feed(b'D');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadId)));
}

// ============================================================================
// Power Control Commands
// ============================================================================

#[test]
fn test_parse_read_power_switch() {
    let mut parser = CatParser::new();
    parser.feed(b'P');
    parser.feed(b'S');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadPowerSwitch)));
}

#[test]
fn test_parse_set_power_switch_on() {
    let mut parser = CatParser::new();
    for c in b"PS1" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetPowerSwitch(true))));
}

#[test]
fn test_parse_set_power_switch_off() {
    let mut parser = CatParser::new();
    for c in b"PS0" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetPowerSwitch(false))));
}

#[test]
fn test_parse_transmit() {
    let mut parser = CatParser::new();
    parser.feed(b'T');
    parser.feed(b'X');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::Transmit(true))));
}

#[test]
fn test_parse_receive() {
    let mut parser = CatParser::new();
    parser.feed(b'R');
    parser.feed(b'X');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::Transmit(false))));
}

// ============================================================================
// Power Level Commands
// ============================================================================

#[test]
fn test_parse_read_power() {
    let mut parser = CatParser::new();
    parser.feed(b'P');
    parser.feed(b'C');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadPower)));
}

#[test]
fn test_parse_set_power_50_percent() {
    let mut parser = CatParser::new();
    for c in b"PC050" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    match cmd {
        Some(CatCommand::SetPower(pwr)) => {
            assert_eq!(pwr.as_percent(), 50);
        }
        _ => panic!("Expected SetPower command"),
    }
}

#[test]
fn test_parse_set_power_100_percent() {
    let mut parser = CatParser::new();
    for c in b"PC100" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    match cmd {
        Some(CatCommand::SetPower(pwr)) => {
            assert_eq!(pwr.as_percent(), 100);
        }
        _ => panic!("Expected SetPower command"),
    }
}

// ============================================================================
// Tuning Commands
// ============================================================================

#[test]
fn test_parse_tune_up() {
    let mut parser = CatParser::new();
    parser.feed(b'U');
    parser.feed(b'P');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::TuneUp)));
}

#[test]
fn test_parse_tune_down() {
    let mut parser = CatParser::new();
    parser.feed(b'D');
    parser.feed(b'N');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::TuneDown)));
}

// ============================================================================
// AGC and Noise Blanker Commands
// ============================================================================

#[test]
fn test_parse_read_agc() {
    let mut parser = CatParser::new();
    parser.feed(b'G');
    parser.feed(b'T');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadAgc)));
}

#[test]
fn test_parse_set_agc() {
    let mut parser = CatParser::new();
    for c in b"GT002" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetAgc(2))));
}

#[test]
fn test_parse_read_nb() {
    let mut parser = CatParser::new();
    parser.feed(b'N');
    parser.feed(b'B');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadNb)));
}

#[test]
fn test_parse_set_nb_on() {
    let mut parser = CatParser::new();
    for c in b"NB1" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetNb(true))));
}

// ============================================================================
// Preamp and Attenuator Commands
// ============================================================================

#[test]
fn test_parse_read_preamp() {
    let mut parser = CatParser::new();
    parser.feed(b'P');
    parser.feed(b'A');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadPreamp)));
}

#[test]
fn test_parse_set_preamp_on() {
    let mut parser = CatParser::new();
    for c in b"PA1" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetPreamp(true))));
}

#[test]
fn test_parse_read_att() {
    let mut parser = CatParser::new();
    parser.feed(b'R');
    parser.feed(b'A');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadAtt)));
}

#[test]
fn test_parse_set_att_on() {
    let mut parser = CatParser::new();
    for c in b"RA01" {
        parser.feed(*c);
    }
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::SetAtt(true))));
}

// ============================================================================
// Unknown Command Tests
// ============================================================================

#[test]
fn test_parse_unknown_command() {
    let mut parser = CatParser::new();
    parser.feed(b'X');
    parser.feed(b'Y');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::Unknown(_))));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_parser_ignores_line_endings() {
    let mut parser = CatParser::new();
    parser.feed(b'F');
    parser.feed(b'\r');
    parser.feed(b'\n');
    parser.feed(b'A');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadFrequency(false))));
}

#[test]
fn test_parser_buffer_overflow_protection() {
    let mut parser = CatParser::new();
    // Feed more than MAX_CMD_LEN bytes to trigger overflow reset
    for _ in 0..100 {
        parser.feed(b'X');
    }
    // Buffer was cleared at overflow, need to start fresh command
    // The 'X' chars would have been cleared, but we need a semicolon
    // to reset properly
    parser.feed(b';'); // Clear any partial state
    parser.feed(b'I');
    parser.feed(b'D');
    let cmd = parser.feed(b';');
    assert!(matches!(cmd, Some(CatCommand::ReadId)));
}

#[test]
fn test_parser_empty_command() {
    let mut parser = CatParser::new();
    let cmd = parser.feed(b';');
    assert!(cmd.is_none());
}

#[test]
fn test_parser_single_char_command() {
    let mut parser = CatParser::new();
    parser.feed(b'F');
    let cmd = parser.feed(b';');
    assert!(cmd.is_none());
}

// ============================================================================
// Response Formatter Tests
// ============================================================================

#[test]
fn test_response_creation() {
    let _resp = CatResponse::new();
}

#[test]
fn test_response_default() {
    let _resp = CatResponse::default();
}

#[test]
fn test_response_frequency_vfo_a() {
    let mut resp = CatResponse::new();
    let freq = Frequency::from_hz(7_074_000).unwrap();
    resp.frequency(freq, false);
    assert_eq!(resp.as_str(), "FA00007074000;");
}

#[test]
fn test_response_frequency_vfo_b() {
    let mut resp = CatResponse::new();
    let freq = Frequency::from_hz(14_070_000).unwrap();
    resp.frequency(freq, true);
    assert_eq!(resp.as_str(), "FB00014070000;");
}

#[test]
fn test_response_mode_usb() {
    let mut resp = CatResponse::new();
    resp.mode(Mode::Usb);
    assert_eq!(resp.as_str(), "MD2;");
}

#[test]
fn test_response_mode_lsb() {
    let mut resp = CatResponse::new();
    resp.mode(Mode::Lsb);
    assert_eq!(resp.as_str(), "MD1;");
}

#[test]
fn test_response_mode_cw() {
    let mut resp = CatResponse::new();
    resp.mode(Mode::Cw);
    assert_eq!(resp.as_str(), "MD3;");
}

#[test]
fn test_response_id() {
    let mut resp = CatResponse::new();
    resp.id();
    assert_eq!(resp.as_str(), "ID019;");
}

#[test]
fn test_response_power() {
    let mut resp = CatResponse::new();
    let pwr = PowerLevel::from_percent(50);
    resp.power(pwr);
    assert_eq!(resp.as_str(), "PC050;");
}

#[test]
fn test_response_power_full() {
    let mut resp = CatResponse::new();
    let pwr = PowerLevel::from_percent(100);
    resp.power(pwr);
    assert_eq!(resp.as_str(), "PC100;");
}

#[test]
fn test_response_status() {
    let mut resp = CatResponse::new();
    let freq = Frequency::from_hz(7_074_000).unwrap();
    resp.status(freq, Mode::Usb, false);
    let status = resp.as_str();
    assert!(status.starts_with("IF00007074000"));
    assert!(status.ends_with(";"));
}

#[test]
fn test_response_clear() {
    let mut resp = CatResponse::new();
    resp.id();
    assert!(!resp.as_str().is_empty());
    resp.clear();
    assert!(resp.as_str().is_empty());
}

#[test]
fn test_response_as_bytes() {
    let mut resp = CatResponse::new();
    resp.id();
    assert_eq!(resp.as_bytes(), b"ID019;");
}

// Note: to_radio_event tests are only available in embedded mode
// as they require the RadioEvent type from crate::radio::state
