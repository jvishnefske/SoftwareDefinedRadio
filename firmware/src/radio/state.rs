//! Radio State Machine
//!
//! Manages the overall state of the radio transceiver.
//! Implements immutable state transitions for predictable behavior.

use crate::types::{Band, Frequency, Mode, PowerLevel, TuningStep, TxRxState};

/// Complete radio state (immutable)
#[derive(Clone, Copy, Debug)]
pub struct RadioState {
    /// Current VFO frequency
    frequency: Frequency,
    /// Operating mode
    mode: Mode,
    /// Tuning step size
    step: TuningStep,
    /// Current band
    band: Option<Band>,
    /// TX/RX state
    txrx: TxRxState,
    /// Power level
    power: PowerLevel,
    /// VFO A/B selection
    pub vfo_select: VfoSelect,
    /// Split operation enabled
    pub split: bool,
    /// RIT (Receiver Incremental Tuning) offset
    rit_offset: i32,
    /// RIT enabled
    rit_enabled: bool,
    /// XIT (Transmitter Incremental Tuning) offset
    xit_offset: i32,
    /// XIT enabled
    xit_enabled: bool,
    /// AGC mode
    agc_mode: AgcMode,
    /// Noise blanker enabled
    noise_blanker: bool,
    /// Preamp enabled
    preamp: bool,
    /// Attenuator enabled
    attenuator: bool,
}

impl RadioState {
    /// Create a new radio state with defaults
    #[must_use]
    pub fn new(frequency: Frequency) -> Self {
        let band = Band::from_frequency(frequency);
        let mode = band.map_or(Mode::Usb, super::super::types::Band::default_mode);

        Self {
            frequency,
            mode,
            step: TuningStep::KHz1,
            band,
            txrx: TxRxState::Rx,
            power: PowerLevel::default(),
            vfo_select: VfoSelect::A,
            split: false,
            rit_offset: 0,
            rit_enabled: false,
            xit_offset: 0,
            xit_enabled: false,
            agc_mode: AgcMode::Medium,
            noise_blanker: false,
            preamp: false,
            attenuator: false,
        }
    }

    /// Get current frequency
    #[must_use]
    pub const fn frequency(&self) -> Frequency {
        self.frequency
    }

    /// Get receive frequency (applies RIT if enabled)
    #[must_use]
    pub fn rx_frequency(&self) -> Frequency {
        if self.rit_enabled {
            let hz = self.frequency.as_hz() as i32 + self.rit_offset;
            Frequency::from_hz(hz.max(0) as u32).unwrap_or(self.frequency)
        } else {
            self.frequency
        }
    }

    /// Get transmit frequency (applies XIT if enabled)
    #[must_use]
    pub fn tx_frequency(&self) -> Frequency {
        if self.xit_enabled {
            let hz = self.frequency.as_hz() as i32 + self.xit_offset;
            Frequency::from_hz(hz.max(0) as u32).unwrap_or(self.frequency)
        } else {
            self.frequency
        }
    }

    /// Get operating mode
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Get tuning step
    #[must_use]
    pub const fn step(&self) -> TuningStep {
        self.step
    }

    /// Get current band
    #[must_use]
    pub const fn band(&self) -> Option<Band> {
        self.band
    }

    /// Get TX/RX state
    #[must_use]
    pub const fn txrx(&self) -> TxRxState {
        self.txrx
    }

    /// Get power level
    #[must_use]
    pub const fn power(&self) -> PowerLevel {
        self.power
    }

    /// Check if transmitting
    #[must_use]
    pub const fn is_transmitting(&self) -> bool {
        matches!(self.txrx, TxRxState::Tx)
    }

    /// Set frequency (returns new state)
    #[must_use]
    pub fn with_frequency(self, frequency: Frequency) -> Self {
        let band = Band::from_frequency(frequency);
        Self {
            frequency,
            band,
            ..self
        }
    }

    /// Tune up (returns new state)
    #[must_use]
    pub fn tune_up(self) -> Self {
        self.with_frequency(self.frequency.tune_up(self.step))
    }

    /// Tune down (returns new state)
    #[must_use]
    pub fn tune_down(self) -> Self {
        self.with_frequency(self.frequency.tune_down(self.step))
    }

    /// Set mode (returns new state)
    #[must_use]
    pub const fn with_mode(self, mode: Mode) -> Self {
        Self { mode, ..self }
    }

    /// Cycle to next mode (returns new state)
    #[must_use]
    pub fn next_mode(self) -> Self {
        let mode = match self.mode {
            Mode::Lsb => Mode::Usb,
            Mode::Usb => Mode::Cw,
            Mode::Cw => Mode::CwR,
            Mode::CwR => Mode::Am,
            Mode::Am => Mode::Fm,
            Mode::Fm => Mode::Lsb,
        };
        Self { mode, ..self }
    }

    /// Set tuning step (returns new state)
    #[must_use]
    pub const fn with_step(self, step: TuningStep) -> Self {
        Self { step, ..self }
    }

    /// Cycle to next larger step (returns new state)
    #[must_use]
    pub fn next_step(self) -> Self {
        Self {
            step: self.step.next_larger(),
            ..self
        }
    }

    /// Set TX/RX state (returns new state)
    #[must_use]
    pub const fn with_txrx(self, txrx: TxRxState) -> Self {
        Self { txrx, ..self }
    }

    /// Set power level (returns new state)
    #[must_use]
    pub const fn with_power(self, power: PowerLevel) -> Self {
        Self { power, ..self }
    }

    /// Toggle RIT (returns new state)
    #[must_use]
    pub const fn toggle_rit(self) -> Self {
        Self {
            rit_enabled: !self.rit_enabled,
            ..self
        }
    }

    /// Set RIT offset (returns new state)
    #[must_use]
    pub const fn with_rit_offset(self, offset: i32) -> Self {
        Self {
            rit_offset: offset,
            ..self
        }
    }

    /// Clear RIT offset (returns new state)
    #[must_use]
    pub const fn clear_rit(self) -> Self {
        Self {
            rit_offset: 0,
            rit_enabled: false,
            ..self
        }
    }

    /// Toggle XIT (returns new state)
    #[must_use]
    pub const fn toggle_xit(self) -> Self {
        Self {
            xit_enabled: !self.xit_enabled,
            ..self
        }
    }

    /// Set AGC mode (returns new state)
    #[must_use]
    pub const fn with_agc(self, agc_mode: AgcMode) -> Self {
        Self { agc_mode, ..self }
    }

    /// Toggle noise blanker (returns new state)
    #[must_use]
    pub const fn toggle_nb(self) -> Self {
        Self {
            noise_blanker: !self.noise_blanker,
            ..self
        }
    }

    /// Toggle preamp (returns new state)
    #[must_use]
    pub const fn toggle_preamp(self) -> Self {
        Self {
            preamp: !self.preamp,
            ..self
        }
    }

    /// Toggle attenuator (returns new state)
    #[must_use]
    pub const fn toggle_attenuator(self) -> Self {
        Self {
            attenuator: !self.attenuator,
            ..self
        }
    }

    /// Get AGC mode
    #[must_use]
    pub const fn agc_mode(&self) -> AgcMode {
        self.agc_mode
    }

    /// Check if noise blanker is enabled
    #[must_use]
    pub const fn noise_blanker_enabled(&self) -> bool {
        self.noise_blanker
    }

    /// Check if preamp is enabled
    #[must_use]
    pub const fn preamp_enabled(&self) -> bool {
        self.preamp
    }

    /// Check if attenuator is enabled
    #[must_use]
    pub const fn attenuator_enabled(&self) -> bool {
        self.attenuator
    }
}

impl Default for RadioState {
    fn default() -> Self {
        Self::new(Frequency::from_hz(7_074_000).unwrap())
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for RadioState {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "Radio({}, {}, {})",
            self.frequency,
            self.mode,
            self.txrx
        );
    }
}

/// VFO A/B selection
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VfoSelect {
    /// VFO A
    #[default]
    A,
    /// VFO B
    B,
}

impl VfoSelect {
    /// Toggle VFO selection
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for VfoSelect {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::A => defmt::write!(f, "VFO-A"),
            Self::B => defmt::write!(f, "VFO-B"),
        }
    }
}

/// AGC mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AgcMode {
    /// AGC off
    Off,
    /// Fast AGC (voice/SSB)
    Fast,
    /// Medium AGC (default)
    #[default]
    Medium,
    /// Slow AGC (CW/data)
    Slow,
}

impl AgcMode {
    /// Get attack time in milliseconds
    #[must_use]
    pub const fn attack_ms(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Fast => 2,
            Self::Medium => 10,
            Self::Slow => 50,
        }
    }

    /// Get decay time in milliseconds
    #[must_use]
    pub const fn decay_ms(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Fast => 100,
            Self::Medium => 500,
            Self::Slow => 2000,
        }
    }

    /// Cycle to next mode
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Off => Self::Fast,
            Self::Fast => Self::Medium,
            Self::Medium => Self::Slow,
            Self::Slow => Self::Off,
        }
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for AgcMode {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Off => defmt::write!(f, "AGC-OFF"),
            Self::Fast => defmt::write!(f, "AGC-F"),
            Self::Medium => defmt::write!(f, "AGC-M"),
            Self::Slow => defmt::write!(f, "AGC-S"),
        }
    }
}

/// Radio event (command) that triggers state transitions
#[derive(Clone, Copy, Debug)]
pub enum RadioEvent {
    /// Tune frequency by step
    Tune(i32),
    /// Set frequency directly
    SetFrequency(Frequency),
    /// Change mode
    SetMode(Mode),
    /// Cycle mode
    NextMode,
    /// Change step size
    SetStep(TuningStep),
    /// Cycle step size
    NextStep,
    /// Start transmit
    StartTx,
    /// Stop transmit
    StopTx,
    /// Set power level
    SetPower(PowerLevel),
    /// Toggle RIT
    ToggleRit,
    /// Adjust RIT
    AdjustRit(i32),
    /// Clear RIT
    ClearRit,
    /// Toggle XIT
    ToggleXit,
    /// Cycle AGC
    CycleAgc,
    /// Toggle noise blanker
    ToggleNb,
    /// Toggle preamp
    TogglePreamp,
    /// Toggle attenuator
    ToggleAtt,
    /// Switch VFO
    SwitchVfo,
    /// Swap VFOs
    SwapVfo,
    /// Copy VFO A to B
    CopyAtoB,
    /// Copy VFO B to A
    CopyBtoA,
}

#[cfg(feature = "embedded")]
impl defmt::Format for RadioEvent {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Tune(steps) => defmt::write!(f, "Tune({})", steps),
            Self::SetFrequency(freq) => defmt::write!(f, "SetFreq({})", freq),
            Self::SetMode(mode) => defmt::write!(f, "SetMode({})", mode),
            Self::NextMode => defmt::write!(f, "NextMode"),
            Self::SetStep(step) => defmt::write!(f, "SetStep({})", step),
            Self::NextStep => defmt::write!(f, "NextStep"),
            Self::StartTx => defmt::write!(f, "StartTx"),
            Self::StopTx => defmt::write!(f, "StopTx"),
            Self::SetPower(pwr) => defmt::write!(f, "SetPower({})", pwr),
            Self::ToggleRit => defmt::write!(f, "ToggleRIT"),
            Self::AdjustRit(hz) => defmt::write!(f, "AdjustRIT({})", hz),
            Self::ClearRit => defmt::write!(f, "ClearRIT"),
            Self::ToggleXit => defmt::write!(f, "ToggleXIT"),
            Self::CycleAgc => defmt::write!(f, "CycleAGC"),
            Self::ToggleNb => defmt::write!(f, "ToggleNB"),
            Self::TogglePreamp => defmt::write!(f, "TogglePreamp"),
            Self::ToggleAtt => defmt::write!(f, "ToggleAtt"),
            Self::SwitchVfo => defmt::write!(f, "SwitchVFO"),
            Self::SwapVfo => defmt::write!(f, "SwapVFO"),
            Self::CopyAtoB => defmt::write!(f, "CopyA>B"),
            Self::CopyBtoA => defmt::write!(f, "CopyB>A"),
        }
    }
}

/// Apply an event to the radio state, returning new state
#[must_use]
pub fn apply_event(state: RadioState, event: RadioEvent) -> RadioState {
    match event {
        RadioEvent::Tune(steps) => {
            if steps > 0 {
                (0..steps).fold(state, |s, _| s.tune_up())
            } else {
                (0..steps.abs()).fold(state, |s, _| s.tune_down())
            }
        }
        RadioEvent::SetFrequency(freq) => state.with_frequency(freq),
        RadioEvent::SetMode(mode) => state.with_mode(mode),
        RadioEvent::NextMode => state.next_mode(),
        RadioEvent::SetStep(step) => state.with_step(step),
        RadioEvent::NextStep => state.next_step(),
        RadioEvent::StartTx => state.with_txrx(TxRxState::Switching),
        RadioEvent::StopTx => state.with_txrx(TxRxState::Switching),
        RadioEvent::SetPower(power) => state.with_power(power),
        RadioEvent::ToggleRit => state.toggle_rit(),
        RadioEvent::AdjustRit(hz) => state.with_rit_offset(state.rit_offset + hz),
        RadioEvent::ClearRit => state.clear_rit(),
        RadioEvent::ToggleXit => state.toggle_xit(),
        RadioEvent::CycleAgc => state.with_agc(state.agc_mode.next()),
        RadioEvent::ToggleNb => state.toggle_nb(),
        RadioEvent::TogglePreamp => state.toggle_preamp(),
        RadioEvent::ToggleAtt => state.toggle_attenuator(),
        RadioEvent::SwitchVfo | RadioEvent::SwapVfo | RadioEvent::CopyAtoB | RadioEvent::CopyBtoA => {
            // VFO operations require VfoManager, handled at higher level
            state
        }
    }
}
