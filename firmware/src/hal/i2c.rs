//! I2C Bus Abstractions
//!
//! Provides async I2C communication for peripherals like `Si5351A` and display.
//! Uses embassy-stm32 async I2C driver with DMA.

use embassy_stm32::i2c::{Error as I2cError, I2c};
use embassy_stm32::mode::Async;

/// I2C operation result
pub type I2cResult<T> = Result<T, I2cError>;

/// I2C device address wrapper
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I2cAddress(u8);

impl I2cAddress {
    /// `Si5351A` clock synthesizer address
    pub const SI5351: Self = Self(0x60);

    /// SSD1306 OLED display address
    pub const SSD1306: Self = Self(0x3C);

    /// Create from 7-bit address
    #[must_use]
    pub const fn new(addr: u8) -> Self {
        Self(addr & 0x7F)
    }

    /// Get the 7-bit address
    #[must_use]
    pub const fn addr(self) -> u8 {
        self.0
    }
}

impl defmt::Format for I2cAddress {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "0x{:02X}", self.0);
    }
}

/// I2C bus wrapper for shared access
pub struct I2cBus<'d> {
    i2c: I2c<'d, Async>,
}

impl<'d> I2cBus<'d> {
    /// Create a new I2C bus wrapper
    #[must_use] 
    pub fn new(i2c: I2c<'d, Async>) -> Self {
        Self { i2c }
    }

    /// Write bytes to a device
    pub async fn write(&mut self, addr: I2cAddress, data: &[u8]) -> I2cResult<()> {
        self.i2c.write(addr.addr(), data).await
    }

    /// Read bytes from a device
    pub async fn read(&mut self, addr: I2cAddress, buffer: &mut [u8]) -> I2cResult<()> {
        self.i2c.read(addr.addr(), buffer).await
    }

    /// Write then read (combined transaction)
    pub async fn write_read(
        &mut self,
        addr: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> I2cResult<()> {
        self.i2c.write_read(addr.addr(), write, read).await
    }

    /// Write a single register
    pub async fn write_reg(&mut self, addr: I2cAddress, reg: u8, value: u8) -> I2cResult<()> {
        self.i2c.write(addr.addr(), &[reg, value]).await
    }

    /// Read a single register
    pub async fn read_reg(&mut self, addr: I2cAddress, reg: u8) -> I2cResult<u8> {
        let mut buf = [0u8];
        self.i2c.write_read(addr.addr(), &[reg], &mut buf).await?;
        Ok(buf[0])
    }

    /// Write multiple registers starting at base address
    pub async fn write_regs(&mut self, addr: I2cAddress, base_reg: u8, values: &[u8]) -> I2cResult<()> {
        // Build buffer with register address prefix
        // For small writes, use stack buffer
        if values.len() <= 16 {
            let mut buf = [0u8; 17];
            buf[0] = base_reg;
            buf[1..=values.len()].copy_from_slice(values);
            self.i2c.write(addr.addr(), &buf[..=values.len()]).await
        } else {
            // For larger writes, do individual register writes
            for (i, &value) in values.iter().enumerate() {
                self.write_reg(addr, base_reg + i as u8, value).await?;
            }
            Ok(())
        }
    }

    /// Read multiple registers starting at base address
    pub async fn read_regs(
        &mut self,
        addr: I2cAddress,
        base_reg: u8,
        buffer: &mut [u8],
    ) -> I2cResult<()> {
        self.i2c.write_read(addr.addr(), &[base_reg], buffer).await
    }

    /// Scan the I2C bus for devices
    pub async fn scan(&mut self) -> heapless::Vec<I2cAddress, 16> {
        let mut devices = heapless::Vec::new();

        for addr in 0x08..0x78 {
            let mut buf = [0u8; 1];
            if self.i2c.read(addr, &mut buf).await.is_ok() {
                let _ = devices.push(I2cAddress::new(addr));
            }
        }

        devices
    }
}

/// I2C device trait for polymorphism
pub trait I2cDevice {
    /// Get the device's I2C address
    fn address(&self) -> I2cAddress;
}

/// Register map helper for devices with many registers
pub struct RegisterMap<const N: usize> {
    /// Shadow copy of register values
    values: [u8; N],
    /// Track which registers are dirty
    dirty: [bool; N],
}

impl<const N: usize> RegisterMap<N> {
    /// Create a new register map initialized to zeros
    #[must_use]
    pub const fn new() -> Self {
        Self {
            values: [0; N],
            dirty: [false; N],
        }
    }

    /// Set a register value (marks dirty)
    pub fn set(&mut self, reg: usize, value: u8) {
        if reg < N && self.values[reg] != value {
            self.values[reg] = value;
            self.dirty[reg] = true;
        }
    }

    /// Get a register value
    #[must_use]
    pub fn get(&self, reg: usize) -> u8 {
        if reg < N {
            self.values[reg]
        } else {
            0
        }
    }

    /// Check if any registers are dirty
    #[must_use]
    pub fn any_dirty(&self) -> bool {
        self.dirty.iter().any(|&d| d)
    }

    /// Mark a register as clean
    pub fn mark_clean(&mut self, reg: usize) {
        if reg < N {
            self.dirty[reg] = false;
        }
    }

    /// Mark all registers as clean
    pub fn mark_all_clean(&mut self) {
        self.dirty.fill(false);
    }

    /// Get iterator over dirty registers
    pub fn dirty_regs(&self) -> impl Iterator<Item = (usize, u8)> + '_ {
        self.dirty
            .iter()
            .enumerate()
            .filter(|(_, &d)| d)
            .map(|(i, _)| (i, self.values[i]))
    }
}

impl<const N: usize> Default for RegisterMap<N> {
    fn default() -> Self {
        Self::new()
    }
}
