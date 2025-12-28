//! SDR Transceiver Main Application
//!
//! Entry point for the STM32G474-based SDR radio firmware.
//! Initializes hardware and spawns async tasks.

#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::i2c::I2c;
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

use sdr_firmware::prelude::*;

// Bind interrupt handlers
bind_interrupts!(struct Irqs {
    I2C1_EV => embassy_stm32::i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => embassy_stm32::i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

/// Main entry point
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("SDR Transceiver Firmware v{}", env!("CARGO_PKG_VERSION"));

    // Initialize STM32G474 peripherals with default clock configuration
    let config = embassy_stm32::Config::default();
    let p = embassy_stm32::init(config);

    info!("Peripherals initialized");

    // Initialize status LED (typically on PA5 for Nucleo boards)
    let led = Output::new(p.PA5, Level::Low, Speed::Low);

    // Initialize I2C1 for Si5351A and other peripherals
    // PB8 = SCL, PB9 = SDA for I2C1 on STM32G474
    let _i2c = I2c::new(
        p.I2C1,
        p.PB8, // SCL
        p.PB9, // SDA
        Irqs,
        p.DMA1_CH1,
        p.DMA1_CH2,
        Hertz(400_000), // 400kHz Fast Mode
        Default::default(),
    );

    info!("I2C1 initialized at 400kHz");

    // Spawn background tasks
    spawner.spawn(heartbeat_task(led)).unwrap();
    // spawner.spawn(radio_control_task()).unwrap();
    // spawner.spawn(dsp_processing_task()).unwrap();
    // spawner.spawn(ui_task()).unwrap();

    info!("Tasks spawned, entering main loop");

    // Main loop - additional coordination can happen here
    loop {
        Timer::after(Duration::from_secs(10)).await;
        info!("Main loop tick");
    }
}

/// Heartbeat task - blinks LED to show system is running
#[embassy_executor::task]
async fn heartbeat_task(mut led: Output<'static>) {
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(100)).await;
        led.set_low();
        Timer::after(Duration::from_millis(900)).await;
    }
}
