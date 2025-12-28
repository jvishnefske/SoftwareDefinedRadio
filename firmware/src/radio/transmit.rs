//! Transmit Control
//!
//! Manages the transmit sequence including T/R switching,
//! SWR protection, and power control.

use crate::types::{PowerLevel, SwrReading, TxRxState};

/// T/R relay switching delay in microseconds
const TR_RELAY_DELAY_US: u32 = 10_000;

/// Transmit state machine
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum TxState {
    /// Radio is receiving
    #[default]
    Rx,
    /// Switching from RX to TX
    SwitchingToTx,
    /// Transmitting
    Tx,
    /// Switching from TX to RX
    SwitchingToRx,
    /// TX inhibited (SWR protection)
    Inhibited,
}

impl TxState {
    /// Convert to `TxRxState`
    #[must_use]
    pub const fn as_txrx(self) -> TxRxState {
        match self {
            Self::Rx => TxRxState::Rx,
            Self::Tx => TxRxState::Tx,
            Self::SwitchingToTx | Self::SwitchingToRx => TxRxState::Switching,
            Self::Inhibited => TxRxState::Rx,
        }
    }
}


#[cfg(feature = "embedded")]
impl defmt::Format for TxState {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Rx => defmt::write!(f, "RX"),
            Self::SwitchingToTx => defmt::write!(f, "RX→TX"),
            Self::Tx => defmt::write!(f, "TX"),
            Self::SwitchingToRx => defmt::write!(f, "TX→RX"),
            Self::Inhibited => defmt::write!(f, "INHIBIT"),
        }
    }
}

/// Transmit controller
#[derive(Clone, Debug)]
pub struct TxController {
    /// Current state
    state: TxState,
    /// PTT input state
    ptt: bool,
    /// VOX trigger state
    vox: bool,
    /// Requested power level
    power: PowerLevel,
    /// Actual power output (may be reduced for SWR)
    actual_power: PowerLevel,
    /// Last SWR reading
    last_swr: Option<SwrReading>,
    /// SWR protection trip count
    swr_trip_count: u32,
    /// T/R switch delay countdown (microseconds)
    switch_delay_us: u32,
    /// TX timeout countdown (seconds)
    timeout_s: u32,
    /// TX timeout limit (0 = disabled)
    timeout_limit_s: u32,
    /// TX inhibit flag
    inhibit: bool,
}

impl TxController {
    /// Default TX timeout (10 minutes)
    pub const DEFAULT_TIMEOUT_S: u32 = 600;

    /// SWR protection threshold
    pub const SWR_LIMIT: f32 = 3.0;

    /// SWR critical threshold (immediate shutoff)
    pub const SWR_CRITICAL: f32 = 5.0;

    /// Create a new transmit controller
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: TxState::Rx,
            ptt: false,
            vox: false,
            power: PowerLevel::default(),
            actual_power: PowerLevel::default(),
            last_swr: None,
            swr_trip_count: 0,
            switch_delay_us: 0,
            timeout_s: 0,
            timeout_limit_s: Self::DEFAULT_TIMEOUT_S,
            inhibit: false,
        }
    }

    /// Get current state
    #[must_use]
    pub const fn state(&self) -> TxState {
        self.state
    }

    /// Get `TxRx` state
    #[must_use]
    pub const fn txrx(&self) -> TxRxState {
        self.state.as_txrx()
    }

    /// Check if transmitting
    #[must_use]
    pub const fn is_transmitting(&self) -> bool {
        matches!(self.state, TxState::Tx)
    }

    /// Check if switching
    #[must_use]
    pub const fn is_switching(&self) -> bool {
        matches!(self.state, TxState::SwitchingToTx | TxState::SwitchingToRx)
    }

    /// Get requested power level
    #[must_use]
    pub const fn power(&self) -> PowerLevel {
        self.power
    }

    /// Get actual power (may be reduced)
    #[must_use]
    pub const fn actual_power(&self) -> PowerLevel {
        self.actual_power
    }

    /// Get last SWR reading
    #[must_use]
    pub const fn last_swr(&self) -> Option<SwrReading> {
        self.last_swr
    }

    /// Get SWR trip count
    #[must_use]
    pub const fn swr_trip_count(&self) -> u32 {
        self.swr_trip_count
    }

    /// Set power level
    pub fn set_power(&mut self, power: PowerLevel) {
        self.power = power;
        if !self.is_transmitting() {
            self.actual_power = power;
        }
    }

    /// Set TX timeout limit (0 = disabled)
    pub fn set_timeout(&mut self, seconds: u32) {
        self.timeout_limit_s = seconds;
    }

    /// Set PTT state
    pub fn set_ptt(&mut self, pressed: bool) {
        self.ptt = pressed;
    }

    /// Set VOX trigger state
    pub fn set_vox(&mut self, triggered: bool) {
        self.vox = triggered;
    }

    /// Set TX inhibit
    pub fn set_inhibit(&mut self, inhibit: bool) {
        self.inhibit = inhibit;
    }

    /// Clear SWR protection trip
    pub fn clear_swr_trip(&mut self) {
        self.swr_trip_count = 0;
        if self.state == TxState::Inhibited {
            self.state = TxState::Rx;
        }
    }

    /// Update with SWR reading
    pub fn update_swr(&mut self, reading: SwrReading) {
        self.last_swr = Some(reading);

        let swr = reading.swr_ratio();

        if swr > Self::SWR_CRITICAL && self.is_transmitting() {
            // Critical SWR - immediate shutdown
            self.state = TxState::Inhibited;
            self.swr_trip_count += 1;
            self.actual_power = PowerLevel::MIN;
        } else if swr > Self::SWR_LIMIT && self.is_transmitting() {
            // High SWR - reduce power
            self.swr_trip_count += 1;
            let reduction = ((swr - Self::SWR_LIMIT) * 10.0) as u8;
            let new_percent = self.power.as_percent().saturating_sub(reduction);
            self.actual_power = PowerLevel::from_percent(new_percent.max(10));
        }
    }

    /// Update state machine (call periodically)
    /// Returns actions to take
    pub fn update(&mut self, elapsed_us: u32) -> TxAction {
        let want_tx = (self.ptt || self.vox) && !self.inhibit;

        match self.state {
            TxState::Rx => {
                if want_tx {
                    self.state = TxState::SwitchingToTx;
                    self.switch_delay_us = TR_RELAY_DELAY_US;
                    return TxAction::EnableTrRelay;
                }
            }

            TxState::SwitchingToTx => {
                if !want_tx {
                    // Aborted before TX started
                    self.state = TxState::SwitchingToRx;
                    return TxAction::DisableTrRelay;
                }

                self.switch_delay_us = self.switch_delay_us.saturating_sub(elapsed_us);
                if self.switch_delay_us == 0 {
                    self.state = TxState::Tx;
                    self.timeout_s = 0;
                    self.actual_power = self.power;
                    return TxAction::EnablePa;
                }
            }

            TxState::Tx => {
                // Check timeout
                if self.timeout_limit_s > 0 && self.timeout_s >= self.timeout_limit_s {
                    self.state = TxState::SwitchingToRx;
                    return TxAction::DisablePa;
                }

                if !want_tx {
                    self.state = TxState::SwitchingToRx;
                    return TxAction::DisablePa;
                }

                // Update power if SWR reduced it
                return TxAction::SetPower(self.actual_power);
            }

            TxState::SwitchingToRx => {
                self.switch_delay_us = self.switch_delay_us.saturating_sub(elapsed_us);
                if self.switch_delay_us == 0 {
                    self.state = TxState::Rx;
                    return TxAction::DisableTrRelay;
                }
            }

            TxState::Inhibited => {
                if !want_tx {
                    self.state = TxState::Rx;
                }
            }
        }

        TxAction::None
    }

    /// Update TX timeout counter (call once per second during TX)
    pub fn tick_timeout(&mut self) {
        if self.is_transmitting() {
            self.timeout_s = self.timeout_s.saturating_add(1);
        }
    }
}

impl Default for TxController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for TxController {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "TxCtrl({}, ptt={}, pwr={})",
            self.state,
            self.ptt,
            self.actual_power
        );
    }
}

/// Action to take from TX controller update
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxAction {
    /// No action needed
    None,
    /// Enable T/R relay for TX
    EnableTrRelay,
    /// Disable T/R relay for RX
    DisableTrRelay,
    /// Enable PA with current power setting
    EnablePa,
    /// Disable PA
    DisablePa,
    /// Set PA power level
    SetPower(PowerLevel),
}

#[cfg(feature = "embedded")]
impl defmt::Format for TxAction {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::None => defmt::write!(f, "None"),
            Self::EnableTrRelay => defmt::write!(f, "EnableTR"),
            Self::DisableTrRelay => defmt::write!(f, "DisableTR"),
            Self::EnablePa => defmt::write!(f, "EnablePA"),
            Self::DisablePa => defmt::write!(f, "DisablePA"),
            Self::SetPower(p) => defmt::write!(f, "SetPower({})", p),
        }
    }
}

/// VOX (Voice Operated Transmit) controller
#[derive(Clone, Copy, Debug)]
pub struct Vox {
    /// VOX enabled
    enabled: bool,
    /// Trigger threshold (0.0-1.0)
    threshold: f32,
    /// Current level
    level: f32,
    /// Hang time in samples
    hang_samples: u32,
    /// Hang counter
    hang_counter: u32,
    /// Anti-trip enabled (suppress speaker audio)
    anti_trip: bool,
}

impl Vox {
    /// Create a new VOX controller
    #[must_use]
    pub const fn new() -> Self {
        Self {
            enabled: false,
            threshold: 0.1,
            level: 0.0,
            hang_samples: 24000, // 500ms at 48kHz
            hang_counter: 0,
            anti_trip: true,
        }
    }

    /// Enable/disable VOX
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.hang_counter = 0;
        }
    }

    /// Set threshold (0.0-1.0)
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.0, 1.0);
    }

    /// Set hang time in milliseconds
    pub fn set_hang_ms(&mut self, ms: u32, sample_rate: u32) {
        self.hang_samples = ms * sample_rate / 1000;
    }

    /// Process audio sample, returns true if TX should be active
    pub fn process(&mut self, audio_level: f32) -> bool {
        if !self.enabled {
            return false;
        }

        // Simple envelope follower
        if audio_level > self.level {
            self.level = audio_level;
        } else {
            self.level *= 0.999; // Slow decay
        }

        if self.level > self.threshold {
            self.hang_counter = self.hang_samples;
            true
        } else if self.hang_counter > 0 {
            self.hang_counter -= 1;
            true
        } else {
            false
        }
    }

    /// Check if VOX is triggered
    #[must_use]
    pub const fn is_triggered(&self) -> bool {
        self.hang_counter > 0
    }

    /// Check if anti-trip is enabled
    #[must_use]
    pub const fn anti_trip(&self) -> bool {
        self.anti_trip
    }

    /// Set anti-trip
    pub fn set_anti_trip(&mut self, enabled: bool) {
        self.anti_trip = enabled;
    }
}

impl Default for Vox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for Vox {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "VOX(en={}, trig={}, level={})",
            self.enabled,
            self.is_triggered(),
            (self.level * 100.0) as u8
        );
    }
}
