//! VFO (Variable Frequency Oscillator) Management
//!
//! Manages dual VFOs (A/B) for split operation and memory channels.

use crate::types::{Band, Frequency, Mode};
use super::state::VfoSelect;

/// VFO settings (stored per VFO)
#[derive(Clone, Copy, Debug)]
pub struct VfoSettings {
    /// VFO frequency
    pub frequency: Frequency,
    /// Operating mode
    pub mode: Mode,
}

impl VfoSettings {
    /// Create new VFO settings
    #[must_use]
    pub const fn new(frequency: Frequency, mode: Mode) -> Self {
        Self { frequency, mode }
    }

    /// Create with auto-detected mode from band
    #[must_use]
    pub fn with_auto_mode(frequency: Frequency) -> Self {
        let mode = Band::from_frequency(frequency)
            .map_or(Mode::Usb, super::super::types::Band::default_mode);
        Self { frequency, mode }
    }
}

impl Default for VfoSettings {
    fn default() -> Self {
        Self::with_auto_mode(Frequency::from_hz(7_074_000).unwrap())
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for VfoSettings {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "VFO({}, {})", self.frequency, self.mode);
    }
}

/// Dual VFO manager
#[derive(Clone, Debug)]
pub struct VfoManager {
    /// VFO A settings
    vfo_a: VfoSettings,
    /// VFO B settings
    vfo_b: VfoSettings,
    /// Currently selected VFO
    selected: VfoSelect,
    /// Split mode enabled
    split: bool,
}

impl VfoManager {
    /// Create a new VFO manager with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            vfo_a: VfoSettings::with_auto_mode(Frequency::from_hz(7_074_000).unwrap()),
            vfo_b: VfoSettings::with_auto_mode(Frequency::from_hz(7_100_000).unwrap()),
            selected: VfoSelect::A,
            split: false,
        }
    }

    /// Get currently selected VFO
    #[must_use]
    pub const fn selected(&self) -> VfoSelect {
        self.selected
    }

    /// Get current VFO settings
    #[must_use]
    pub const fn current(&self) -> &VfoSettings {
        match self.selected {
            VfoSelect::A => &self.vfo_a,
            VfoSelect::B => &self.vfo_b,
        }
    }

    /// Get current VFO settings mutably
    fn current_mut(&mut self) -> &mut VfoSettings {
        match self.selected {
            VfoSelect::A => &mut self.vfo_a,
            VfoSelect::B => &mut self.vfo_b,
        }
    }

    /// Get VFO A settings
    #[must_use]
    pub const fn vfo_a(&self) -> &VfoSettings {
        &self.vfo_a
    }

    /// Get VFO B settings
    #[must_use]
    pub const fn vfo_b(&self) -> &VfoSettings {
        &self.vfo_b
    }

    /// Get receive VFO (always selected VFO)
    #[must_use]
    pub const fn rx_vfo(&self) -> &VfoSettings {
        self.current()
    }

    /// Get transmit VFO (other VFO if split enabled)
    #[must_use]
    pub const fn tx_vfo(&self) -> &VfoSettings {
        if self.split {
            match self.selected {
                VfoSelect::A => &self.vfo_b,
                VfoSelect::B => &self.vfo_a,
            }
        } else {
            self.current()
        }
    }

    /// Check if split mode is enabled
    #[must_use]
    pub const fn split(&self) -> bool {
        self.split
    }

    /// Switch to VFO A
    pub fn select_a(&mut self) {
        self.selected = VfoSelect::A;
    }

    /// Switch to VFO B
    pub fn select_b(&mut self) {
        self.selected = VfoSelect::B;
    }

    /// Toggle VFO selection
    pub fn toggle(&mut self) {
        self.selected = self.selected.toggle();
    }

    /// Swap VFO A and B
    pub fn swap(&mut self) {
        core::mem::swap(&mut self.vfo_a, &mut self.vfo_b);
    }

    /// Copy current VFO to other
    pub fn copy_to_other(&mut self) {
        match self.selected {
            VfoSelect::A => self.vfo_b = self.vfo_a,
            VfoSelect::B => self.vfo_a = self.vfo_b,
        }
    }

    /// Copy VFO A to VFO B
    pub fn copy_a_to_b(&mut self) {
        self.vfo_b = self.vfo_a;
    }

    /// Copy VFO B to VFO A
    pub fn copy_b_to_a(&mut self) {
        self.vfo_a = self.vfo_b;
    }

    /// Set frequency on current VFO
    pub fn set_frequency(&mut self, frequency: Frequency) {
        self.current_mut().frequency = frequency;
    }

    /// Set mode on current VFO
    pub fn set_mode(&mut self, mode: Mode) {
        self.current_mut().mode = mode;
    }

    /// Enable split mode
    pub fn enable_split(&mut self) {
        self.split = true;
    }

    /// Disable split mode
    pub fn disable_split(&mut self) {
        self.split = false;
    }

    /// Toggle split mode
    pub fn toggle_split(&mut self) {
        self.split = !self.split;
    }
}

impl Default for VfoManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for VfoManager {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "VFOMgr(sel={}, A={}, B={}, split={})",
            self.selected,
            self.vfo_a,
            self.vfo_b,
            self.split
        );
    }
}

/// Memory channel storage
#[derive(Clone, Copy, Debug)]
pub struct MemoryChannel {
    /// Channel number
    pub number: u8,
    /// Stored frequency
    pub frequency: Frequency,
    /// Stored mode
    pub mode: Mode,
    /// Channel name (8 chars max)
    pub name: [u8; 8],
    /// Channel is in use
    pub active: bool,
}

impl MemoryChannel {
    /// Create an empty memory channel
    #[must_use]
    pub const fn empty(number: u8) -> Self {
        Self {
            number,
            frequency: Frequency::from_hz_const(7_000_000),
            mode: Mode::Lsb,
            name: [0; 8],
            active: false,
        }
    }

    /// Store VFO settings to channel
    pub fn store(&mut self, settings: &VfoSettings) {
        self.frequency = settings.frequency;
        self.mode = settings.mode;
        self.active = true;
    }

    /// Recall channel to VFO settings
    #[must_use]
    pub const fn recall(&self) -> Option<VfoSettings> {
        if self.active {
            Some(VfoSettings {
                frequency: self.frequency,
                mode: self.mode,
            })
        } else {
            None
        }
    }

    /// Clear the channel
    pub fn clear(&mut self) {
        self.active = false;
        self.name = [0; 8];
    }

    /// Set channel name
    pub fn set_name(&mut self, name: &[u8]) {
        let len = name.len().min(8);
        self.name[..len].copy_from_slice(&name[..len]);
        if len < 8 {
            self.name[len..].fill(0);
        }
    }
}

#[cfg(feature = "embedded")]
impl defmt::Format for MemoryChannel {
    fn format(&self, f: defmt::Formatter) {
        if self.active {
            defmt::write!(f, "M{:02}({}, {})", self.number, self.frequency, self.mode);
        } else {
            defmt::write!(f, "M{:02}(empty)", self.number);
        }
    }
}

/// Memory bank (100 channels)
pub struct MemoryBank {
    channels: [MemoryChannel; 100],
}

impl MemoryBank {
    /// Create a new empty memory bank
    #[must_use]
    pub fn new() -> Self {
        let channels = core::array::from_fn(|i| MemoryChannel::empty(i as u8));
        Self { channels }
    }

    /// Get channel by number
    #[must_use]
    pub fn get(&self, number: u8) -> Option<&MemoryChannel> {
        self.channels.get(number as usize)
    }

    /// Get channel mutably by number
    pub fn get_mut(&mut self, number: u8) -> Option<&mut MemoryChannel> {
        self.channels.get_mut(number as usize)
    }

    /// Store to channel
    pub fn store(&mut self, number: u8, settings: &VfoSettings) -> bool {
        if let Some(ch) = self.get_mut(number) {
            ch.store(settings);
            true
        } else {
            false
        }
    }

    /// Recall from channel
    #[must_use]
    pub fn recall(&self, number: u8) -> Option<VfoSettings> {
        self.get(number).and_then(MemoryChannel::recall)
    }

    /// Find next active channel
    #[must_use]
    pub fn next_active(&self, from: u8) -> Option<u8> {
        let start = (from as usize + 1) % 100;
        for i in 0..100 {
            let idx = (start + i) % 100;
            if self.channels[idx].active {
                return Some(idx as u8);
            }
        }
        None
    }

    /// Find previous active channel
    #[must_use]
    pub fn prev_active(&self, from: u8) -> Option<u8> {
        let start = if from == 0 { 99 } else { from as usize - 1 };
        for i in 0..100 {
            let idx = (start + 100 - i) % 100;
            if self.channels[idx].active {
                return Some(idx as u8);
            }
        }
        None
    }

    /// Count active channels
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.channels.iter().filter(|ch| ch.active).count()
    }
}

impl Default for MemoryBank {
    fn default() -> Self {
        Self::new()
    }
}

// Helper for const frequency creation
impl Frequency {
    /// Create frequency at compile time (panics if out of range)
    #[must_use]
    pub const fn from_hz_const(hz: u32) -> Self {
        match Self::from_hz(hz) {
            Some(f) => f,
            None => panic!("Frequency out of range"),
        }
    }
}
