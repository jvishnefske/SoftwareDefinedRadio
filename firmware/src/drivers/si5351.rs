//! `Si5351A` Clock Synthesizer Driver
//!
//! Provides frequency synthesis for the SDR transceiver LO.
//! Supports quadrature output generation for QSD/QSE operation.
//!
//! The `Si5351A` generates three independent clock outputs from a single
//! 25MHz crystal reference using fractional PLLs and multisynth dividers.

use crate::hal::i2c::{I2cAddress, I2cBus, I2cResult};
use crate::types::Frequency;
use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;

/// `Si5351A` register addresses
mod reg {
    pub const DEVICE_STATUS: u8 = 0;
    pub const OUTPUT_ENABLE: u8 = 3;
    pub const CLK0_CONTROL: u8 = 16;
    pub const CLK1_CONTROL: u8 = 17;
    pub const CLK2_CONTROL: u8 = 18;
    pub const PLLA_PARAMS: u8 = 26;
    pub const PLLB_PARAMS: u8 = 34;
    pub const MS0_PARAMS: u8 = 42;
    pub const MS1_PARAMS: u8 = 50;
    pub const MS2_PARAMS: u8 = 58;
    pub const CLK0_PHASE: u8 = 165;
    pub const CLK1_PHASE: u8 = 166;
    pub const PLL_RESET: u8 = 177;
    pub const CRYSTAL_LOAD: u8 = 183;
}

/// Clock output identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockOutput {
    /// CLK0 output
    Clk0,
    /// CLK1 output
    Clk1,
    /// CLK2 output
    Clk2,
}

impl ClockOutput {
    /// Get the control register for this output
    const fn control_reg(self) -> u8 {
        match self {
            Self::Clk0 => reg::CLK0_CONTROL,
            Self::Clk1 => reg::CLK1_CONTROL,
            Self::Clk2 => reg::CLK2_CONTROL,
        }
    }

    /// Get the multisynth parameter base register
    const fn ms_reg(self) -> u8 {
        match self {
            Self::Clk0 => reg::MS0_PARAMS,
            Self::Clk1 => reg::MS1_PARAMS,
            Self::Clk2 => reg::MS2_PARAMS,
        }
    }

    /// Get the output enable bit
    const fn enable_bit(self) -> u8 {
        match self {
            Self::Clk0 => 0,
            Self::Clk1 => 1,
            Self::Clk2 => 2,
        }
    }
}

impl defmt::Format for ClockOutput {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::Clk0 => defmt::write!(f, "CLK0"),
            Self::Clk1 => defmt::write!(f, "CLK1"),
            Self::Clk2 => defmt::write!(f, "CLK2"),
        }
    }
}

/// Drive strength setting
#[derive(Clone, Copy, Debug, Default)]
pub enum DriveStrength {
    /// 2mA drive
    Drive2mA,
    /// 4mA drive
    Drive4mA,
    /// 6mA drive
    Drive6mA,
    /// 8mA drive (maximum)
    #[default]
    Drive8mA,
}

impl DriveStrength {
    /// Get register value
    const fn as_reg(self) -> u8 {
        match self {
            Self::Drive2mA => 0,
            Self::Drive4mA => 1,
            Self::Drive6mA => 2,
            Self::Drive8mA => 3,
        }
    }
}

/// PLL source selection
#[derive(Clone, Copy, Debug, Default)]
pub enum PllSource {
    /// Use PLL A
    #[default]
    PllA,
    /// Use PLL B
    PllB,
}

/// Crystal load capacitance
#[derive(Clone, Copy, Debug, Default)]
pub enum CrystalLoad {
    /// 6 pF load
    Load6pF,
    /// 8 pF load
    Load8pF,
    /// 10 pF load
    #[default]
    Load10pF,
}

impl CrystalLoad {
    const fn as_reg(self) -> u8 {
        match self {
            Self::Load6pF => 0b01000000,
            Self::Load8pF => 0b10000000,
            Self::Load10pF => 0b11000000,
        }
    }
}

/// PLL parameters for frequency calculation
#[derive(Clone, Copy, Debug)]
struct PllParams {
    /// Integer part (15-90)
    a: u32,
    /// Numerator (0 to c-1)
    b: u32,
    /// Denominator (1-1048575)
    c: u32,
}


/// Multisynth divider parameters
#[derive(Clone, Copy, Debug)]
struct MsParams {
    /// Integer part (4, 6-1800)
    a: u32,
    /// Numerator
    b: u32,
    /// Denominator
    c: u32,
    /// R divider (1, 2, 4, 8, 16, 32, 64, 128)
    r_div: u8,
}

/// `Si5351A` driver
pub struct Si5351<'d> {
    bus: I2cBus<'d>,
    xtal_freq: u32,
    output_enable: u8,
}

impl<'d> Si5351<'d> {
    /// Default crystal frequency (25 MHz)
    pub const DEFAULT_XTAL: u32 = 25_000_000;

    /// Create a new `Si5351A` driver
    #[must_use] 
    pub fn new(i2c: I2c<'d, Async>) -> Self {
        Self {
            bus: I2cBus::new(i2c),
            xtal_freq: Self::DEFAULT_XTAL,
            output_enable: 0xFF, // All outputs disabled
        }
    }

    /// Initialize the `Si5351A`
    pub async fn init(&mut self, load: CrystalLoad) -> I2cResult<()> {
        // Wait for device to be ready
        self.wait_ready().await?;

        // Disable all outputs during configuration
        self.bus
            .write_reg(I2cAddress::SI5351, reg::OUTPUT_ENABLE, 0xFF)
            .await?;

        // Set crystal load capacitance
        self.bus
            .write_reg(I2cAddress::SI5351, reg::CRYSTAL_LOAD, load.as_reg())
            .await?;

        // Power down all clock outputs
        for clk in [ClockOutput::Clk0, ClockOutput::Clk1, ClockOutput::Clk2] {
            self.bus
                .write_reg(I2cAddress::SI5351, clk.control_reg(), 0x80)
                .await?;
        }

        Ok(())
    }

    /// Wait for device to be ready (`SYS_INIT` cleared)
    async fn wait_ready(&mut self) -> I2cResult<()> {
        for _ in 0..100 {
            let status = self.bus.read_reg(I2cAddress::SI5351, reg::DEVICE_STATUS).await?;
            if status & 0x80 == 0 {
                return Ok(());
            }
            embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
        }
        // Timeout, but continue anyway
        Ok(())
    }

    /// Set frequency on a clock output
    pub async fn set_frequency(&mut self, output: ClockOutput, freq: Frequency) -> I2cResult<()> {
        let freq_hz = freq.as_hz();

        // For QSD, we need 4x the LO frequency
        let target_hz = u64::from(freq_hz);

        // Calculate PLL and MS parameters
        let (pll, ms) = self.calculate_params(target_hz);

        // Program PLL A
        self.program_pll(PllSource::PllA, &pll).await?;

        // Program multisynth
        self.program_multisynth(output, &ms).await?;

        // Configure clock control
        let control = 0x0F | (DriveStrength::Drive8mA.as_reg());
        self.bus
            .write_reg(I2cAddress::SI5351, output.control_reg(), control)
            .await?;

        // Reset PLL
        self.bus
            .write_reg(I2cAddress::SI5351, reg::PLL_RESET, 0x20)
            .await?;

        Ok(())
    }

    /// Set quadrature output (CLK0 and CLK1 with 90° phase)
    pub async fn set_quadrature(&mut self, freq: Frequency) -> I2cResult<()> {
        let freq_hz = freq.as_hz();
        let target_hz = u64::from(freq_hz) * 4; // 4x for QSD

        // Calculate parameters
        let (pll, ms) = self.calculate_params(target_hz);

        // Program PLL A
        self.program_pll(PllSource::PllA, &pll).await?;

        // Program both multisynths with same parameters
        self.program_multisynth(ClockOutput::Clk0, &ms).await?;
        self.program_multisynth(ClockOutput::Clk1, &ms).await?;

        // Set 90 degree phase offset on CLK1
        // Phase = (VCO / Fout) / 4 = ms.a / 4
        let phase = (ms.a / 4) as u8;
        self.bus
            .write_reg(I2cAddress::SI5351, reg::CLK1_PHASE, phase)
            .await?;
        self.bus
            .write_reg(I2cAddress::SI5351, reg::CLK0_PHASE, 0)
            .await?;

        // Configure both outputs
        let control = 0x0F | (DriveStrength::Drive8mA.as_reg());
        self.bus
            .write_reg(I2cAddress::SI5351, ClockOutput::Clk0.control_reg(), control)
            .await?;
        self.bus
            .write_reg(I2cAddress::SI5351, ClockOutput::Clk1.control_reg(), control)
            .await?;

        // Reset PLL to synchronize outputs
        self.bus
            .write_reg(I2cAddress::SI5351, reg::PLL_RESET, 0xA0)
            .await?;

        Ok(())
    }

    /// Enable a clock output
    pub async fn enable(&mut self, output: ClockOutput) -> I2cResult<()> {
        self.output_enable &= !(1 << output.enable_bit());
        self.bus
            .write_reg(I2cAddress::SI5351, reg::OUTPUT_ENABLE, self.output_enable)
            .await
    }

    /// Disable a clock output
    pub async fn disable(&mut self, output: ClockOutput) -> I2cResult<()> {
        self.output_enable |= 1 << output.enable_bit();
        self.bus
            .write_reg(I2cAddress::SI5351, reg::OUTPUT_ENABLE, self.output_enable)
            .await
    }

    /// Enable quadrature outputs (CLK0 and CLK1)
    pub async fn enable_quadrature(&mut self) -> I2cResult<()> {
        self.output_enable &= !0x03; // Enable CLK0 and CLK1
        self.bus
            .write_reg(I2cAddress::SI5351, reg::OUTPUT_ENABLE, self.output_enable)
            .await
    }

    /// Calculate PLL and multisynth parameters for target frequency
    fn calculate_params(&self, target_hz: u64) -> (PllParams, MsParams) {
        // VCO range: 600-900 MHz
        // Try to find integer multisynth divisor first

        // Start with VCO at 900 MHz
        let vco = 900_000_000u64;

        // Calculate multisynth divisor
        let ms_div = vco / target_hz;
        let ms_a = ms_div.clamp(4, 1800) as u32;

        // Calculate actual VCO needed
        let actual_vco = target_hz * u64::from(ms_a);

        // Calculate PLL multiplier from crystal
        let pll_mult = actual_vco / u64::from(self.xtal_freq);
        let pll_a = pll_mult.clamp(15, 90) as u32;

        // For now, use integer division (b=0, c=1)
        // TODO: Implement fractional synthesis for finer tuning

        let pll = PllParams {
            a: pll_a,
            b: 0,
            c: 1,
        };

        let ms = MsParams {
            a: ms_a,
            b: 0,
            c: 1,
            r_div: 0,
        };

        (pll, ms)
    }

    /// Program PLL registers
    async fn program_pll(&mut self, pll: PllSource, params: &PllParams) -> I2cResult<()> {
        let base = match pll {
            PllSource::PllA => reg::PLLA_PARAMS,
            PllSource::PllB => reg::PLLB_PARAMS,
        };

        // Calculate register values
        let p1 = 128 * params.a + ((128 * params.b) / params.c) - 512;
        let p2 = 128 * params.b - params.c * ((128 * params.b) / params.c);
        let p3 = params.c;

        let regs = [
            ((p3 >> 8) & 0xFF) as u8,
            (p3 & 0xFF) as u8,
            ((p1 >> 16) & 0x03) as u8,
            ((p1 >> 8) & 0xFF) as u8,
            (p1 & 0xFF) as u8,
            (((p3 >> 12) & 0xF0) | ((p2 >> 16) & 0x0F)) as u8,
            ((p2 >> 8) & 0xFF) as u8,
            (p2 & 0xFF) as u8,
        ];

        self.bus
            .write_regs(I2cAddress::SI5351, base, &regs)
            .await
    }

    /// Program multisynth registers
    async fn program_multisynth(&mut self, output: ClockOutput, params: &MsParams) -> I2cResult<()> {
        let base = output.ms_reg();

        // Calculate register values
        let p1 = 128 * params.a + ((128 * params.b) / params.c) - 512;
        let p2 = 128 * params.b - params.c * ((128 * params.b) / params.c);
        let p3 = params.c;

        let regs = [
            ((p3 >> 8) & 0xFF) as u8,
            (p3 & 0xFF) as u8,
            ((params.r_div << 4) | ((p1 >> 16) as u8 & 0x03)),
            ((p1 >> 8) & 0xFF) as u8,
            (p1 & 0xFF) as u8,
            (((p3 >> 12) & 0xF0) | ((p2 >> 16) & 0x0F)) as u8,
            ((p2 >> 8) & 0xFF) as u8,
            (p2 & 0xFF) as u8,
        ];

        self.bus
            .write_regs(I2cAddress::SI5351, base, &regs)
            .await
    }
}
