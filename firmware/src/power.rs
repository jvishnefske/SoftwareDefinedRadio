//! Power Management
//!
//! Battery monitoring, thermal management, and power control.

/// Battery voltage reading
#[derive(Clone, Copy, Debug)]
pub struct BatteryVoltage {
    /// Raw ADC reading (12-bit)
    raw: u16,
    /// Voltage divider ratio
    divider_ratio: f32,
    /// Reference voltage
    vref: f32,
}

impl BatteryVoltage {
    /// Create from ADC reading
    #[must_use]
    pub const fn from_adc(raw: u16, divider_ratio: f32, vref: f32) -> Self {
        Self {
            raw,
            divider_ratio,
            vref,
        }
    }

    /// Get voltage in volts
    #[must_use]
    pub fn voltage(&self) -> f32 {
        (f32::from(self.raw) / 4095.0) * self.vref * self.divider_ratio
    }

    /// Get battery percentage (for `LiPo` 3.0-4.2V per cell)
    #[must_use]
    pub fn percentage(&self, cells: u8) -> u8 {
        let v = self.voltage();
        let v_per_cell = v / f32::from(cells);

        // LiPo discharge curve approximation
        let pct = if v_per_cell >= 4.2 {
            100.0
        } else if v_per_cell <= 3.0 {
            0.0
        } else {
            ((v_per_cell - 3.0) / 1.2) * 100.0
        };

        pct as u8
    }

    /// Check if battery is low
    #[must_use]
    pub fn is_low(&self, cells: u8) -> bool {
        self.voltage() / f32::from(cells) < 3.3
    }

    /// Check if battery is critical
    #[must_use]
    pub fn is_critical(&self, cells: u8) -> bool {
        self.voltage() / f32::from(cells) < 3.1
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for BatteryVoltage {
    fn format(&self, f: defmt::Formatter) {
        let v = self.voltage();
        let whole = v as u32;
        let frac = ((v - whole as f32) * 100.0) as u32;
        defmt::write!(f, "{}.{:02}V", whole, frac);
    }
}

/// Temperature reading
#[derive(Clone, Copy, Debug)]
pub struct Temperature {
    /// Temperature in 0.1°C units
    raw_tenths: i16,
}

impl Temperature {
    /// Create from Celsius
    #[must_use]
    pub fn from_celsius(celsius: f32) -> Self {
        Self {
            raw_tenths: (celsius * 10.0) as i16,
        }
    }

    /// Create from raw tenths of a degree
    #[must_use]
    pub const fn from_tenths(tenths: i16) -> Self {
        Self { raw_tenths: tenths }
    }

    /// Get temperature in Celsius
    #[must_use]
    pub fn celsius(&self) -> f32 {
        f32::from(self.raw_tenths) / 10.0
    }

    /// Get temperature in Fahrenheit
    #[must_use]
    pub fn fahrenheit(&self) -> f32 {
        self.celsius() * 9.0 / 5.0 + 32.0
    }

    /// Check if over temperature limit
    #[must_use]
    pub fn is_over_temp(&self, limit_celsius: f32) -> bool {
        self.celsius() > limit_celsius
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for Temperature {
    fn format(&self, f: defmt::Formatter) {
        let c = self.celsius();
        let whole = c as i32;
        let frac = ((c - whole as f32).abs() * 10.0) as u32;
        defmt::write!(f, "{}.{}°C", whole, frac);
    }
}

/// Power state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PowerState {
    /// Running on battery
    #[default]
    Battery,
    /// USB powered
    UsbPowered,
    /// External DC power
    DcPowered,
    /// Low power / sleep mode
    LowPower,
}

#[cfg(feature = "embedded")]
impl defmt::Format for PowerState {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Battery => defmt::write!(f, "BAT"),
            Self::UsbPowered => defmt::write!(f, "USB"),
            Self::DcPowered => defmt::write!(f, "DC"),
            Self::LowPower => defmt::write!(f, "LP"),
        }
    }
}

/// Power manager
#[derive(Clone, Debug)]
pub struct PowerManager {
    /// Current power state
    state: PowerState,
    /// Battery voltage
    battery: Option<BatteryVoltage>,
    /// Number of battery cells
    cells: u8,
    /// PA temperature
    pa_temp: Option<Temperature>,
    /// MCU temperature
    mcu_temp: Option<Temperature>,
    /// TX power limit due to thermal
    thermal_limit_percent: u8,
    /// Over temperature threshold
    over_temp_threshold: f32,
}

impl PowerManager {
    /// Create a new power manager
    #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self {
            state: PowerState::Battery,
            battery: None,
            cells,
            pa_temp: None,
            mcu_temp: None,
            thermal_limit_percent: 100,
            over_temp_threshold: 70.0,
        }
    }

    /// Get current power state
    #[must_use]
    pub const fn state(&self) -> PowerState {
        self.state
    }

    /// Get battery voltage
    #[must_use]
    pub const fn battery(&self) -> Option<BatteryVoltage> {
        self.battery
    }

    /// Get battery percentage
    #[must_use]
    pub fn battery_percent(&self) -> Option<u8> {
        self.battery.map(|b| b.percentage(self.cells))
    }

    /// Get PA temperature
    #[must_use]
    pub const fn pa_temp(&self) -> Option<Temperature> {
        self.pa_temp
    }

    /// Get thermal power limit
    #[must_use]
    pub const fn thermal_limit(&self) -> u8 {
        self.thermal_limit_percent
    }

    /// Update battery voltage
    pub fn update_battery(&mut self, voltage: BatteryVoltage) {
        self.battery = Some(voltage);

        // Check for critical battery
        if voltage.is_critical(self.cells) && self.state == PowerState::Battery {
            self.state = PowerState::LowPower;
        }
    }

    /// Update PA temperature
    pub fn update_pa_temp(&mut self, temp: Temperature) {
        self.pa_temp = Some(temp);

        // Thermal limiting
        let celsius = temp.celsius();
        if celsius > self.over_temp_threshold {
            self.thermal_limit_percent = 0;
        } else if celsius > self.over_temp_threshold - 10.0 {
            // Linear ramp down
            let over = celsius - (self.over_temp_threshold - 10.0);
            self.thermal_limit_percent = (100.0 - over * 10.0) as u8;
        } else {
            self.thermal_limit_percent = 100;
        }
    }

    /// Update MCU temperature
    pub fn update_mcu_temp(&mut self, temp: Temperature) {
        self.mcu_temp = Some(temp);
    }

    /// Set power state
    pub fn set_state(&mut self, state: PowerState) {
        self.state = state;
    }

    /// Check if TX is allowed
    #[must_use]
    pub fn tx_allowed(&self) -> bool {
        // Don't allow TX on low battery
        if let Some(batt) = self.battery {
            if batt.is_critical(self.cells) {
                return false;
            }
        }

        // Don't allow TX if over temperature
        if self.thermal_limit_percent == 0 {
            return false;
        }

        true
    }

    /// Get effective power limit (0-100)
    #[must_use]
    pub fn effective_power_limit(&self) -> u8 {
        let mut limit = self.thermal_limit_percent;

        // Reduce power on low battery
        if let Some(batt) = self.battery {
            if batt.is_low(self.cells) {
                limit = limit.min(50);
            }
        }

        limit
    }
}

impl Default for PowerManager {
    fn default() -> Self {
        Self::new(1) // Single cell default
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for PowerManager {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Power({}, limit={}%)", self.state, self.thermal_limit_percent);
    }
}
