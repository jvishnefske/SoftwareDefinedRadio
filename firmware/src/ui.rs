//! User Interface
//!
//! Display rendering and menu system for the SDR transceiver.

use crate::drivers::display::{DisplayBuffer, StatusRenderer};
use crate::drivers::encoder::{Direction, EncoderEvent};
use crate::radio::state::RadioState;
use crate::types::{Frequency, Mode};

/// UI screen/mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Screen {
    /// Main operating screen
    #[default]
    Main,
    /// Menu screen
    Menu,
    /// VFO editing
    VfoEdit,
    /// Memory channels
    Memory,
    /// Settings
    Settings,
    /// Band scope (if display allows)
    Scope,
}

impl defmt::Format for Screen {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Main => defmt::write!(f, "Main"),
            Self::Menu => defmt::write!(f, "Menu"),
            Self::VfoEdit => defmt::write!(f, "VFOEdit"),
            Self::Memory => defmt::write!(f, "Memory"),
            Self::Settings => defmt::write!(f, "Settings"),
            Self::Scope => defmt::write!(f, "Scope"),
        }
    }
}

/// Menu item
#[derive(Clone, Copy, Debug)]
pub struct MenuItem {
    /// Item label
    pub label: &'static str,
    /// Item action or submenu
    pub action: MenuAction,
}

/// Menu action
#[derive(Clone, Copy, Debug)]
pub enum MenuAction {
    /// Go to screen
    GoTo(Screen),
    /// Toggle boolean setting
    Toggle(&'static str),
    /// Adjust numeric value
    Adjust(&'static str, i32, i32), // min, max
    /// Execute function
    Execute(&'static str),
    /// Back to previous menu
    Back,
}

/// Main menu items
pub const MAIN_MENU: &[MenuItem] = &[
    MenuItem {
        label: "Mode",
        action: MenuAction::GoTo(Screen::Main),
    },
    MenuItem {
        label: "Band",
        action: MenuAction::GoTo(Screen::Main),
    },
    MenuItem {
        label: "Memory",
        action: MenuAction::GoTo(Screen::Memory),
    },
    MenuItem {
        label: "Settings",
        action: MenuAction::GoTo(Screen::Settings),
    },
    MenuItem {
        label: "Back",
        action: MenuAction::Back,
    },
];

/// UI state
#[derive(Clone, Debug)]
pub struct UiState {
    /// Current screen
    screen: Screen,
    /// Previous screen (for back navigation)
    prev_screen: Screen,
    /// Menu selection index
    menu_index: usize,
    /// S-meter level (0-100)
    s_meter: u8,
    /// SWR value
    swr: f32,
    /// Update flags
    needs_update: bool,
}

impl UiState {
    /// Create new UI state
    #[must_use]
    pub const fn new() -> Self {
        Self {
            screen: Screen::Main,
            prev_screen: Screen::Main,
            menu_index: 0,
            s_meter: 0,
            swr: 1.0,
            needs_update: true,
        }
    }

    /// Get current screen
    #[must_use]
    pub const fn screen(&self) -> Screen {
        self.screen
    }

    /// Set screen
    pub fn set_screen(&mut self, screen: Screen) {
        self.prev_screen = self.screen;
        self.screen = screen;
        self.menu_index = 0;
        self.needs_update = true;
    }

    /// Go back to previous screen
    pub fn go_back(&mut self) {
        self.screen = self.prev_screen;
        self.needs_update = true;
    }

    /// Update S-meter
    pub fn set_s_meter(&mut self, level: u8) {
        if self.s_meter != level {
            self.s_meter = level;
            self.needs_update = true;
        }
    }

    /// Update SWR
    pub fn set_swr(&mut self, swr: f32) {
        if (self.swr - swr).abs() > 0.1 {
            self.swr = swr;
            self.needs_update = true;
        }
    }

    /// Check if display needs update
    #[must_use]
    pub const fn needs_update(&self) -> bool {
        self.needs_update
    }

    /// Mark as updated
    pub fn mark_updated(&mut self) {
        self.needs_update = false;
    }

    /// Force update
    pub fn invalidate(&mut self) {
        self.needs_update = true;
    }

    /// Handle encoder event
    pub fn handle_encoder(&mut self, event: EncoderEvent) -> Option<UiAction> {
        match self.screen {
            Screen::Main => self.handle_main_encoder(event),
            Screen::Menu => self.handle_menu_encoder(event),
            _ => None,
        }
    }

    fn handle_main_encoder(&mut self, event: EncoderEvent) -> Option<UiAction> {
        match event {
            EncoderEvent::Rotate { direction, steps } => {
                let delta = match direction {
                    Direction::Clockwise => steps as i32,
                    Direction::CounterClockwise => -(steps as i32),
                };
                Some(UiAction::Tune(delta))
            }
            EncoderEvent::ButtonPress => Some(UiAction::NextStep),
            EncoderEvent::LongPress => {
                self.set_screen(Screen::Menu);
                None
            }
            _ => None,
        }
    }

    fn handle_menu_encoder(&mut self, event: EncoderEvent) -> Option<UiAction> {
        match event {
            EncoderEvent::Rotate { direction, .. } => {
                match direction {
                    Direction::Clockwise => {
                        self.menu_index = (self.menu_index + 1) % MAIN_MENU.len();
                    }
                    Direction::CounterClockwise => {
                        self.menu_index = if self.menu_index == 0 {
                            MAIN_MENU.len() - 1
                        } else {
                            self.menu_index - 1
                        };
                    }
                }
                self.needs_update = true;
                None
            }
            EncoderEvent::ButtonPress => {
                let item = &MAIN_MENU[self.menu_index];
                match item.action {
                    MenuAction::GoTo(screen) => {
                        self.set_screen(screen);
                    }
                    MenuAction::Back => {
                        self.go_back();
                    }
                    MenuAction::Execute(cmd) => {
                        return Some(UiAction::Execute(cmd));
                    }
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// UI action (result of user interaction)
#[derive(Clone, Copy, Debug)]
pub enum UiAction {
    /// Tune by steps
    Tune(i32),
    /// Set frequency directly
    SetFrequency(Frequency),
    /// Change mode
    SetMode(Mode),
    /// Next tuning step
    NextStep,
    /// Toggle PTT
    TogglePtt,
    /// Execute command by name
    Execute(&'static str),
}

impl defmt::Format for UiAction {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Tune(steps) => defmt::write!(f, "Tune({})", steps),
            Self::SetFrequency(freq) => defmt::write!(f, "SetFreq({})", freq),
            Self::SetMode(mode) => defmt::write!(f, "SetMode({})", mode),
            Self::NextStep => defmt::write!(f, "NextStep"),
            Self::TogglePtt => defmt::write!(f, "TogglePtt"),
            Self::Execute(cmd) => defmt::write!(f, "Exec({})", cmd),
        }
    }
}

/// Render the main screen
pub fn render_main_screen(buffer: &mut DisplayBuffer, state: &RadioState, ui: &UiState) {
    buffer.clear();

    // Render band
    StatusRenderer::render_band(buffer, state.band());

    // Render mode
    StatusRenderer::render_mode(buffer, state.mode());

    // Render TX/RX
    StatusRenderer::render_txrx(buffer, state.txrx());

    // Render frequency
    StatusRenderer::render_frequency(buffer, state.frequency());

    // Render tuning step
    StatusRenderer::render_step(buffer, state.step());

    // Render S-meter
    StatusRenderer::render_smeter(buffer, ui.s_meter);

    // Render SWR if transmitting
    if state.is_transmitting() {
        StatusRenderer::render_swr(buffer, ui.swr);
    }
}

/// Render the menu screen
pub fn render_menu_screen(buffer: &mut DisplayBuffer, menu_index: usize) {
    use embedded_graphics::mono_font::ascii::FONT_6X10;
    use embedded_graphics::mono_font::MonoTextStyle;
    use embedded_graphics::pixelcolor::BinaryColor;
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
    use embedded_graphics::text::{Baseline, Text};

    buffer.clear();

    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let inv_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::Off);

    // Title
    let _ = Text::with_baseline("MENU", Point::new(50, 0), style, Baseline::Top).draw(buffer);

    // Menu items
    for (i, item) in MAIN_MENU.iter().enumerate() {
        let y = 14 + i as i32 * 10;

        if i == menu_index {
            // Highlight selected item
            let rect = Rectangle::new(Point::new(0, y - 1), Size::new(128, 10));
            let _ = rect
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(buffer);
            let _ =
                Text::with_baseline(item.label, Point::new(4, y), inv_style, Baseline::Top)
                    .draw(buffer);
        } else {
            let _ =
                Text::with_baseline(item.label, Point::new(4, y), style, Baseline::Top)
                    .draw(buffer);
        }
    }
}
