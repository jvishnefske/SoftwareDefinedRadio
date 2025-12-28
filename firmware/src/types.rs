//! Shared types used across the SDR firmware
//!
//! This module defines domain-specific types that enforce invariants
//! at compile time and provide type safety throughout the codebase.

use core::fmt;
#[cfg(feature = "embedded")]
use micromath::F32Ext;

/// Frequency in Hertz with validation
///
/// Represents a valid frequency within the supported range.
/// The frequency is stored in Hz for precision.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frequency(u32);

impl Frequency {
    /// Minimum supported frequency (3.5 MHz for 80m band)
    pub const MIN_HZ: u32 = 3_500_000;

    /// Maximum supported frequency (21.45 MHz for 15m band)
    pub const MAX_HZ: u32 = 21_450_000;

    /// Create a new Frequency from Hz, returns None if out of range
    #[must_use]
    pub const fn from_hz(hz: u32) -> Option<Self> {
        if hz >= Self::MIN_HZ && hz <= Self::MAX_HZ {
            Some(Self(hz))
        } else {
            None
        }
    }

    /// Create a new Frequency from kHz
    #[must_use]
    pub const fn from_khz(khz: u32) -> Option<Self> {
        Self::from_hz(khz * 1000)
    }

    /// Get the frequency in Hz
    #[must_use]
    pub const fn as_hz(self) -> u32 {
        self.0
    }

    /// Get the frequency in kHz (truncated)
    #[must_use]
    pub const fn as_khz(self) -> u32 {
        self.0 / 1000
    }

    /// Get the frequency in MHz as floating point
    #[must_use]
    pub fn as_mhz_f32(self) -> f32 {
        self.0 as f32 / 1_000_000.0
    }

    /// Create frequency for 4x LO (quadrature sampling)
    #[must_use]
    pub const fn as_4x_lo(self) -> u32 {
        self.0 * 4
    }

    /// Tune up by a step amount
    #[must_use]
    pub fn tune_up(self, step: TuningStep) -> Self {
        let new_hz = self.0.saturating_add(step.as_hz());
        Self::from_hz(new_hz).unwrap_or(Self(Self::MAX_HZ))
    }

    /// Tune down by a step amount
    #[must_use]
    pub fn tune_down(self, step: TuningStep) -> Self {
        let new_hz = self.0.saturating_sub(step.as_hz());
        Self::from_hz(new_hz).unwrap_or(Self(Self::MIN_HZ))
    }
}

impl fmt::Debug for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frequency({} Hz)", self.0)
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for Frequency {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{} Hz", self.0);
    }
}

/// Tuning step size
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuningStep {
    /// 1 Hz step
    Hz1,
    /// 10 Hz step
    Hz10,
    /// 100 Hz step
    Hz100,
    /// 1 kHz step
    KHz1,
    /// 10 kHz step
    KHz10,
    /// 100 kHz step
    KHz100,
    /// 1 MHz step
    MHz1,
}

impl TuningStep {
    /// Get the step size in Hz
    #[must_use]
    pub const fn as_hz(self) -> u32 {
        match self {
            Self::Hz1 => 1,
            Self::Hz10 => 10,
            Self::Hz100 => 100,
            Self::KHz1 => 1_000,
            Self::KHz10 => 10_000,
            Self::KHz100 => 100_000,
            Self::MHz1 => 1_000_000,
        }
    }

    /// Cycle to next larger step
    #[must_use]
    pub const fn next_larger(self) -> Self {
        match self {
            Self::Hz1 => Self::Hz10,
            Self::Hz10 => Self::Hz100,
            Self::Hz100 => Self::KHz1,
            Self::KHz1 => Self::KHz10,
            Self::KHz10 => Self::KHz100,
            Self::KHz100 => Self::MHz1,
            Self::MHz1 => Self::Hz1, // Wrap around
        }
    }

    /// Cycle to next smaller step
    #[must_use]
    pub const fn next_smaller(self) -> Self {
        match self {
            Self::Hz1 => Self::MHz1, // Wrap around
            Self::Hz10 => Self::Hz1,
            Self::Hz100 => Self::Hz10,
            Self::KHz1 => Self::Hz100,
            Self::KHz10 => Self::KHz1,
            Self::KHz100 => Self::KHz10,
            Self::MHz1 => Self::KHz100,
        }
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for TuningStep {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Hz1 => defmt::write!(f, "1 Hz"),
            Self::Hz10 => defmt::write!(f, "10 Hz"),
            Self::Hz100 => defmt::write!(f, "100 Hz"),
            Self::KHz1 => defmt::write!(f, "1 kHz"),
            Self::KHz10 => defmt::write!(f, "10 kHz"),
            Self::KHz100 => defmt::write!(f, "100 kHz"),
            Self::MHz1 => defmt::write!(f, "1 MHz"),
        }
    }
}

/// Operating mode for the radio
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// Lower Sideband (voice below 10 MHz)
    #[default]
    Lsb,
    /// Upper Sideband (voice above 10 MHz)
    Usb,
    /// Continuous Wave (Morse code)
    Cw,
    /// CW with reverse sideband
    CwR,
    /// Amplitude Modulation
    Am,
    /// Frequency Modulation (narrow)
    Fm,
}

impl Mode {
    /// Get the audio filter bandwidth for this mode
    #[must_use]
    pub const fn bandwidth_hz(self) -> u32 {
        match self {
            Self::Lsb | Self::Usb => 2700,
            Self::Cw | Self::CwR => 500,
            Self::Am => 6000,
            Self::Fm => 12000,
        }
    }

    /// Get the BFO offset for this mode
    #[must_use]
    pub const fn bfo_offset_hz(self) -> i32 {
        match self {
            Self::Lsb => 1500,
            Self::Usb => -1500,
            Self::Cw => -700,
            Self::CwR => 700,
            Self::Am | Self::Fm => 0,
        }
    }

    /// Check if this mode uses sideband inversion
    #[must_use]
    pub const fn inverted_sideband(self) -> bool {
        matches!(self, Self::Lsb | Self::CwR)
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for Mode {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Lsb => defmt::write!(f, "LSB"),
            Self::Usb => defmt::write!(f, "USB"),
            Self::Cw => defmt::write!(f, "CW"),
            Self::CwR => defmt::write!(f, "CW-R"),
            Self::Am => defmt::write!(f, "AM"),
            Self::Fm => defmt::write!(f, "FM"),
        }
    }
}

/// Amateur radio band definition
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    /// 80 meters (3.5 - 4.0 MHz)
    M80,
    /// 40 meters (7.0 - 7.3 MHz)
    M40,
    /// 30 meters (10.1 - 10.15 MHz)
    M30,
    /// 20 meters (14.0 - 14.35 MHz)
    M20,
    /// 17 meters (18.068 - 18.168 MHz)
    M17,
    /// 15 meters (21.0 - 21.45 MHz)
    M15,
}

impl Band {
    /// Get the band for a given frequency
    #[must_use]
    pub const fn from_frequency(freq: Frequency) -> Option<Self> {
        let hz = freq.as_hz();
        if hz >= 3_500_000 && hz <= 4_000_000 {
            Some(Self::M80)
        } else if hz >= 7_000_000 && hz <= 7_300_000 {
            Some(Self::M40)
        } else if hz >= 10_100_000 && hz <= 10_150_000 {
            Some(Self::M30)
        } else if hz >= 14_000_000 && hz <= 14_350_000 {
            Some(Self::M20)
        } else if hz >= 18_068_000 && hz <= 18_168_000 {
            Some(Self::M17)
        } else if hz >= 21_000_000 && hz <= 21_450_000 {
            Some(Self::M15)
        } else {
            None
        }
    }

    /// Get the band start frequency
    #[must_use]
    pub const fn start_hz(self) -> u32 {
        match self {
            Self::M80 => 3_500_000,
            Self::M40 => 7_000_000,
            Self::M30 => 10_100_000,
            Self::M20 => 14_000_000,
            Self::M17 => 18_068_000,
            Self::M15 => 21_000_000,
        }
    }

    /// Get the band end frequency
    #[must_use]
    pub const fn end_hz(self) -> u32 {
        match self {
            Self::M80 => 4_000_000,
            Self::M40 => 7_300_000,
            Self::M30 => 10_150_000,
            Self::M20 => 14_350_000,
            Self::M17 => 18_168_000,
            Self::M15 => 21_450_000,
        }
    }

    /// Get the LPF relay index for this band (0-4)
    #[must_use]
    pub const fn lpf_index(self) -> u8 {
        match self {
            Self::M80 => 0,
            Self::M40 => 1,
            Self::M30 | Self::M20 => 2,
            Self::M17 => 3,
            Self::M15 => 4,
        }
    }

    /// Get the default mode for this band
    #[must_use]
    pub const fn default_mode(self) -> Mode {
        match self {
            Self::M80 | Self::M40 => Mode::Lsb,
            Self::M30 | Self::M20 | Self::M17 | Self::M15 => Mode::Usb,
        }
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for Band {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::M80 => defmt::write!(f, "80m"),
            Self::M40 => defmt::write!(f, "40m"),
            Self::M30 => defmt::write!(f, "30m"),
            Self::M20 => defmt::write!(f, "20m"),
            Self::M17 => defmt::write!(f, "17m"),
            Self::M15 => defmt::write!(f, "15m"),
        }
    }
}

/// Power level setting
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerLevel(u8);

impl PowerLevel {
    /// Minimum power (1 mW)
    pub const MIN: Self = Self(0);

    /// Maximum power (5W)
    pub const MAX: Self = Self(100);

    /// Create a power level from percentage (0-100)
    #[must_use]
    pub const fn from_percent(percent: u8) -> Self {
        if percent > 100 {
            Self(100)
        } else {
            Self(percent)
        }
    }

    /// Get the power level as a percentage
    #[must_use]
    pub const fn as_percent(self) -> u8 {
        self.0
    }

    /// Calculate PWM duty cycle for PA drive
    #[must_use]
    pub const fn as_pwm_duty(self) -> u16 {
        // Map 0-100% to PWM range (0-65535)
        (self.0 as u16) * 655
    }
}

impl Default for PowerLevel {
    fn default() -> Self {
        Self(50) // 50% power as default
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for PowerLevel {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{}%", self.0);
    }
}

/// SWR measurement result
#[derive(Clone, Copy, Debug)]
pub struct SwrReading {
    /// Forward power in arbitrary ADC units
    pub forward: u16,
    /// Reflected power in arbitrary ADC units
    pub reflected: u16,
}

impl SwrReading {
    /// Calculate SWR ratio
    #[must_use]
    pub fn swr_ratio(&self) -> f32 {
        if self.forward == 0 {
            return 999.0;
        }

        let rho = f32::from(self.reflected) / f32::from(self.forward);
        let rho = rho.sqrt().min(0.99); // Clamp reflection coefficient

        (1.0 + rho) / (1.0 - rho)
    }

    /// Check if SWR is acceptable for transmit
    #[must_use]
    pub fn is_acceptable(&self) -> bool {
        self.swr_ratio() < 3.0
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for SwrReading {
    fn format(&self, f: defmt::Formatter) {
        let swr = self.swr_ratio();
        if swr > 99.0 {
            defmt::write!(f, "SWR: >99:1");
        } else {
            // Format as fixed point since we don't have float formatting
            let whole = swr as u32;
            let frac = ((swr - whole as f32) * 10.0) as u32;
            defmt::write!(f, "SWR: {}.{}:1", whole, frac);
        }
    }
}

/// Transmit/Receive state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TxRxState {
    /// Receiving
    #[default]
    Rx,
    /// Transmitting
    Tx,
    /// Transitioning (relay switching)
    Switching,
}

#[cfg(feature = "embedded")]
impl defmt::Format for TxRxState {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Rx => defmt::write!(f, "RX"),
            Self::Tx => defmt::write!(f, "TX"),
            Self::Switching => defmt::write!(f, "SWITCHING"),
        }
    }
}
