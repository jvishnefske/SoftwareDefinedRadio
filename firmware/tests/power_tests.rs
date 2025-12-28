//! Power Management Tests
//!
//! Tests for battery monitoring, thermal management, and power control.
//! Run with: cargo test --target x86_64-unknown-linux-gnu --no-default-features --features std --test power_tests

use sdr_firmware::power::{BatteryVoltage, PowerManager, PowerState, Temperature};

// =============================================================================
// Battery Voltage Tests
// =============================================================================

#[test]
fn battery_voltage_from_adc() {
    // 4S LiPo at 16.8V (full charge)
    // Divider ratio 11:1, Vref 3.3V
    // ADC = (16.8 / 11) / 3.3 * 4095 = 1895
    let batt = BatteryVoltage::from_adc(1895, 11.0, 3.3);
    let v = batt.voltage();
    assert!((v - 16.8).abs() < 0.2, "Expected ~16.8V, got {}", v);
}

#[test]
fn battery_voltage_empty() {
    // ADC reading of 0
    let batt = BatteryVoltage::from_adc(0, 11.0, 3.3);
    assert_eq!(batt.voltage(), 0.0);
}

#[test]
fn battery_voltage_percentage_full() {
    // 4S at 16.8V = 100%
    let batt = BatteryVoltage::from_adc(1895, 11.0, 3.3);
    let pct = batt.percentage(4);
    assert!(pct >= 95, "Full battery should be ~100%, got {}%", pct);
}

#[test]
fn battery_voltage_percentage_empty() {
    // 4S at 12.0V (3.0V per cell) = 0%
    let batt = BatteryVoltage::from_adc(1354, 11.0, 3.3);
    let pct = batt.percentage(4);
    assert!(pct <= 5, "Empty battery should be ~0%, got {}%", pct);
}

#[test]
fn battery_voltage_percentage_midpoint() {
    // 4S at 14.4V (3.6V per cell) = ~50%
    let batt = BatteryVoltage::from_adc(1624, 11.0, 3.3);
    let pct = batt.percentage(4);
    assert!(pct >= 40 && pct <= 60, "Midpoint should be ~50%, got {}%", pct);
}

#[test]
fn battery_voltage_is_low() {
    // 4S at 13.0V (3.25V per cell) - low but not critical
    let batt = BatteryVoltage::from_adc(1467, 11.0, 3.3);
    assert!(batt.is_low(4), "3.25V/cell should be low");
}

#[test]
fn battery_voltage_is_not_low() {
    // 4S at 15.0V (3.75V per cell) - healthy
    let batt = BatteryVoltage::from_adc(1693, 11.0, 3.3);
    assert!(!batt.is_low(4), "3.75V/cell should not be low");
}

#[test]
fn battery_voltage_is_critical() {
    // 4S at 12.2V (3.05V per cell) - critical
    let batt = BatteryVoltage::from_adc(1377, 11.0, 3.3);
    assert!(batt.is_critical(4), "3.05V/cell should be critical");
}

#[test]
fn battery_voltage_is_not_critical() {
    // 4S at 13.0V (3.25V per cell) - low but not critical
    let batt = BatteryVoltage::from_adc(1467, 11.0, 3.3);
    assert!(!batt.is_critical(4), "3.25V/cell should not be critical");
}

// =============================================================================
// Temperature Tests
// =============================================================================

#[test]
fn temperature_from_celsius() {
    let temp = Temperature::from_celsius(25.0);
    assert!((temp.celsius() - 25.0).abs() < 0.1);
}

#[test]
fn temperature_from_tenths() {
    let temp = Temperature::from_tenths(250);
    assert!((temp.celsius() - 25.0).abs() < 0.1);
}

#[test]
fn temperature_negative() {
    let temp = Temperature::from_celsius(-10.5);
    assert!((temp.celsius() - (-10.5)).abs() < 0.1);
}

#[test]
fn temperature_fahrenheit() {
    let temp = Temperature::from_celsius(0.0);
    assert!((temp.fahrenheit() - 32.0).abs() < 0.1);

    let temp = Temperature::from_celsius(100.0);
    assert!((temp.fahrenheit() - 212.0).abs() < 0.1);
}

#[test]
fn temperature_is_over_temp() {
    let temp = Temperature::from_celsius(75.0);
    assert!(temp.is_over_temp(70.0));
    assert!(!temp.is_over_temp(80.0));
}

// =============================================================================
// Power State Tests
// =============================================================================

#[test]
fn power_state_default() {
    let state = PowerState::default();
    assert_eq!(state, PowerState::Battery);
}

#[test]
fn power_state_equality() {
    assert_eq!(PowerState::Battery, PowerState::Battery);
    assert_ne!(PowerState::Battery, PowerState::UsbPowered);
}

// =============================================================================
// Power Manager Tests
// =============================================================================

#[test]
fn power_manager_new() {
    let pm = PowerManager::new(4);
    assert_eq!(pm.state(), PowerState::Battery);
    assert!(pm.battery().is_none());
    assert_eq!(pm.thermal_limit(), 100);
}

#[test]
fn power_manager_default() {
    let pm = PowerManager::default();
    assert_eq!(pm.state(), PowerState::Battery);
}

#[test]
fn power_manager_update_battery() {
    let mut pm = PowerManager::new(4);
    let batt = BatteryVoltage::from_adc(1700, 11.0, 3.3);
    pm.update_battery(batt);

    assert!(pm.battery().is_some());
    assert!(pm.battery_percent().is_some());
}

#[test]
fn power_manager_critical_battery_triggers_low_power() {
    let mut pm = PowerManager::new(4);
    pm.set_state(PowerState::Battery);

    // Critical battery (3.0V per cell)
    let batt = BatteryVoltage::from_adc(1354, 11.0, 3.3);
    pm.update_battery(batt);

    assert_eq!(pm.state(), PowerState::LowPower);
}

#[test]
fn power_manager_thermal_limiting_off() {
    let mut pm = PowerManager::new(4);

    // Cool PA
    let temp = Temperature::from_celsius(50.0);
    pm.update_pa_temp(temp);

    assert_eq!(pm.thermal_limit(), 100);
}

#[test]
fn power_manager_thermal_limiting_ramp() {
    let mut pm = PowerManager::new(4);

    // Warm PA (65°C with 70°C threshold = 50% limit)
    let temp = Temperature::from_celsius(65.0);
    pm.update_pa_temp(temp);

    let limit = pm.thermal_limit();
    assert!(limit < 100 && limit > 0, "Should be ramping, got {}%", limit);
}

#[test]
fn power_manager_thermal_shutdown() {
    let mut pm = PowerManager::new(4);

    // Overheated PA
    let temp = Temperature::from_celsius(85.0);
    pm.update_pa_temp(temp);

    assert_eq!(pm.thermal_limit(), 0);
}

#[test]
fn power_manager_tx_allowed_normal() {
    let mut pm = PowerManager::new(4);

    // Healthy battery
    let batt = BatteryVoltage::from_adc(1700, 11.0, 3.3);
    pm.update_battery(batt);

    assert!(pm.tx_allowed());
}

#[test]
fn power_manager_tx_blocked_critical_battery() {
    let mut pm = PowerManager::new(4);

    // Critical battery
    let batt = BatteryVoltage::from_adc(1354, 11.0, 3.3);
    pm.update_battery(batt);

    assert!(!pm.tx_allowed());
}

#[test]
fn power_manager_tx_blocked_overtemp() {
    let mut pm = PowerManager::new(4);

    // Overheated
    let temp = Temperature::from_celsius(85.0);
    pm.update_pa_temp(temp);

    assert!(!pm.tx_allowed());
}

#[test]
fn power_manager_effective_limit_low_battery() {
    let mut pm = PowerManager::new(4);

    // Low (but not critical) battery - should limit to 50%
    let batt = BatteryVoltage::from_adc(1467, 11.0, 3.3);
    pm.update_battery(batt);

    let limit = pm.effective_power_limit();
    assert!(limit <= 50, "Low battery should limit power, got {}%", limit);
}

#[test]
fn power_manager_effective_limit_thermal_and_battery() {
    let mut pm = PowerManager::new(4);

    // Low battery AND warm PA - should take minimum
    let batt = BatteryVoltage::from_adc(1467, 11.0, 3.3);
    pm.update_battery(batt);

    let temp = Temperature::from_celsius(66.0); // ~40% thermal limit
    pm.update_pa_temp(temp);

    let limit = pm.effective_power_limit();
    assert!(limit <= 50, "Should take min of thermal and battery limits, got {}%", limit);
}

#[test]
fn power_manager_set_state() {
    let mut pm = PowerManager::new(4);

    pm.set_state(PowerState::UsbPowered);
    assert_eq!(pm.state(), PowerState::UsbPowered);

    pm.set_state(PowerState::DcPowered);
    assert_eq!(pm.state(), PowerState::DcPowered);
}

#[test]
fn power_manager_mcu_temp() {
    let mut pm = PowerManager::new(4);

    let temp = Temperature::from_celsius(45.0);
    pm.update_mcu_temp(temp);

    // MCU temp doesn't affect thermal limiting (only PA temp does)
    assert_eq!(pm.thermal_limit(), 100);
}

#[test]
fn power_manager_pa_temp_accessor() {
    let mut pm = PowerManager::new(4);

    assert!(pm.pa_temp().is_none());

    let temp = Temperature::from_celsius(55.0);
    pm.update_pa_temp(temp);

    assert!(pm.pa_temp().is_some());
    let pa = pm.pa_temp().unwrap();
    assert!((pa.celsius() - 55.0).abs() < 0.1);
}
