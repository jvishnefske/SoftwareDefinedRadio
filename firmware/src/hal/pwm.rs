//! PWM Driver
//!
//! Provides PWM output for PA drive control and Class-E H-bridge.
//! Uses advanced timer features for complementary outputs with dead time.

// PWM abstractions for embedded systems

/// PWM duty cycle (0-65535)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DutyCycle(u16);

impl DutyCycle {
    /// 0% duty cycle
    pub const ZERO: Self = Self(0);

    /// 100% duty cycle
    pub const FULL: Self = Self(65535);

    /// 50% duty cycle
    pub const HALF: Self = Self(32768);

    /// Create from 16-bit value
    #[must_use]
    pub const fn from_raw(value: u16) -> Self {
        Self(value)
    }

    /// Create from percentage (0-100)
    #[must_use]
    pub fn from_percent(percent: u8) -> Self {
        let value = (u32::from(percent.min(100)) * 65535) / 100;
        Self(value as u16)
    }

    /// Create from fraction (0.0-1.0)
    #[must_use]
    pub fn from_fraction(frac: f32) -> Self {
        let clamped = frac.clamp(0.0, 1.0);
        Self((clamped * 65535.0) as u16)
    }

    /// Get raw 16-bit value
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Get as percentage
    #[must_use]
    pub fn as_percent(self) -> u8 {
        ((u32::from(self.0) * 100) / 65535) as u8
    }

    /// Get as fraction
    #[must_use]
    pub fn as_fraction(self) -> f32 {
        f32::from(self.0) / 65535.0
    }

    /// Scale by another duty cycle
    #[must_use]
    pub fn scale(self, other: Self) -> Self {
        let product = (u32::from(self.0) * u32::from(other.0)) / 65535;
        Self(product as u16)
    }
}

impl Default for DutyCycle {
    fn default() -> Self {
        Self::ZERO
    }
}

impl defmt::Format for DutyCycle {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{}%", self.as_percent());
    }
}

/// PA drive controller
///
/// Controls the Class-E PA drive level using PWM.
pub struct PaDrive {
    /// Current duty cycle
    duty: DutyCycle,
    /// Maximum allowed duty (for protection)
    max_duty: DutyCycle,
}

impl PaDrive {
    /// Create PA drive controller with safety limit
    #[must_use]
    pub const fn new() -> Self {
        Self {
            duty: DutyCycle::ZERO,
            max_duty: DutyCycle::from_raw(52428), // 80% max for safety
        }
    }

    /// Set drive level (respects max limit)
    pub fn set(&mut self, duty: DutyCycle) -> DutyCycle {
        self.duty = if duty.0 > self.max_duty.0 {
            self.max_duty
        } else {
            duty
        };
        self.duty
    }

    /// Set drive level from percentage
    pub fn set_percent(&mut self, percent: u8) -> DutyCycle {
        self.set(DutyCycle::from_percent(percent))
    }

    /// Get current duty cycle
    #[must_use]
    pub const fn duty(&self) -> DutyCycle {
        self.duty
    }

    /// Reduce drive for SWR protection
    pub fn reduce_for_swr(&mut self, swr: f32) {
        if swr > 2.0 {
            // Reduce power proportionally
            let factor = 2.0 / swr;
            let new_duty = DutyCycle::from_fraction(self.duty.as_fraction() * factor);
            self.duty = new_duty;
        }
    }

    /// Emergency stop - zero drive immediately
    pub fn stop(&mut self) {
        self.duty = DutyCycle::ZERO;
    }
}

impl Default for PaDrive {
    fn default() -> Self {
        Self::new()
    }
}

/// H-bridge phase configuration for Class-E operation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum HBridgePhase {
    /// Both sides off
    #[default]
    Off,
    /// A high, B low (positive half)
    AHighBLow,
    /// A low, B high (negative half)
    ALowBHigh,
    /// Both sides off (zero crossing)
    ZeroCrossing,
}

impl HBridgePhase {
    /// Get the next phase in sequence (for RF generation)
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Off => Self::AHighBLow,
            Self::AHighBLow => Self::ZeroCrossing,
            Self::ZeroCrossing => Self::ALowBHigh,
            Self::ALowBHigh => Self::Off,
        }
    }
}


impl defmt::Format for HBridgePhase {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Off => defmt::write!(f, "OFF"),
            Self::AHighBLow => defmt::write!(f, "A+B-"),
            Self::ALowBHigh => defmt::write!(f, "A-B+"),
            Self::ZeroCrossing => defmt::write!(f, "ZC"),
        }
    }
}

/// Dead time configuration for H-bridge
#[derive(Clone, Copy, Debug)]
pub struct DeadTime {
    /// Dead time in nanoseconds
    ns: u32,
}

impl DeadTime {
    /// No dead time (dangerous!)
    pub const ZERO: Self = Self { ns: 0 };

    /// Minimum safe dead time (50ns)
    pub const MINIMUM: Self = Self { ns: 50 };

    /// Default safe dead time (100ns)
    pub const DEFAULT: Self = Self { ns: 100 };

    /// Create from nanoseconds
    #[must_use]
    pub const fn from_ns(ns: u32) -> Self {
        Self { ns }
    }

    /// Get dead time in nanoseconds
    #[must_use]
    pub const fn as_ns(self) -> u32 {
        self.ns
    }

    /// Calculate timer register value for given clock
    #[must_use]
    pub fn as_timer_value(self, timer_clock_hz: u32) -> u8 {
        // DTG = dead_time * timer_clock / 1e9
        let ticks = (u64::from(self.ns) * u64::from(timer_clock_hz)) / 1_000_000_000;
        ticks.min(255) as u8
    }
}

impl Default for DeadTime {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl defmt::Format for DeadTime {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{}ns", self.ns);
    }
}

/// Complementary PWM output state
#[derive(Clone, Copy, Debug, Default)]
pub struct ComplementaryPwm {
    /// Main output duty cycle
    pub main: DutyCycle,
    /// Complementary output is automatically inverted
    pub dead_time: DeadTime,
}

impl ComplementaryPwm {
    /// Create new complementary PWM configuration
    #[must_use]
    pub const fn new() -> Self {
        Self {
            main: DutyCycle::ZERO,
            dead_time: DeadTime::DEFAULT,
        }
    }

    /// Set duty cycle for 50% square wave
    pub fn set_square_wave(&mut self) {
        self.main = DutyCycle::HALF;
    }
}
