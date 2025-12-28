# SDR Transceiver Functional Requirements

## Project: STM32G474-Based SDR Transceiver
**Version:** 1.0
**Status:** Active Development

---

## Requirements Traceability Matrix

### Functional Requirements (FR)

| ID | Requirement | Priority | Status | Validated | Test Method |
|----|-------------|----------|--------|-----------|-------------|
| FR-001 | Transmit and receive on 80m, 40m, 30m, 20m, 17m, 15m bands | Must | [ ] | [ ] | RF test |
| FR-002 | Support CW, LSB, USB, AM modes | Must | [ ] | [ ] | Functional test |
| FR-003 | Adjustable output power 0.5W to 5W | Must | [ ] | [ ] | Power meter |
| FR-004 | Display frequency on OLED (128x64) | Must | [ ] | [ ] | Visual |
| FR-005 | Rotary encoder for frequency tuning | Must | [ ] | [ ] | Functional |
| FR-006 | PTT input for transmit control | Must | [ ] | [ ] | Functional |
| FR-007 | CAT control via USB CDC (TS-480 protocol) | Should | [ ] | [ ] | WSJT-X test |
| FR-008 | USB Audio for digital modes (48kHz mono) | Should | [ ] | [ ] | WSJT-X test |
| FR-009 | Si5351A clock synthesis for LO generation | Must | [ ] | [ ] | Frequency counter |
| FR-010 | SWR measurement and protection | Must | [ ] | [ ] | RF test |

### Power Requirements (PR)

| ID | Requirement | Priority | Status | Validated | Test Method |
|----|-------------|----------|--------|-----------|-------------|
| PR-001 | USB-PD negotiation up to 20V/3A via TPS25750D | Must | [ ] | [ ] | PD analyzer |
| PR-002 | 4S Li-Ion charging via BQ25798 (16.8V CV) | Must | [ ] | [ ] | Charge cycle |
| PR-003 | Battery protection via BQ76920 (OVP/UVP/OCP/OTP) | Must | [ ] | [ ] | Fault test |
| PR-004 | 5V buck via XL1509 from VSYS (up to 18V input) | Must | [ ] | [ ] | Voltage test |
| PR-005 | 3.3V LDO via AMS1117-3.3 from 5V rail | Must | [ ] | [ ] | Voltage test |
| PR-006 | Battery state monitoring via I2C | Should | [ ] | [ ] | Telemetry |
| PR-007 | Dead battery boot from USB-C | Must | [ ] | [ ] | Boot test |

### Performance Requirements (PF)

| ID | Requirement | Value | Status | Validated | Test Method |
|----|-------------|-------|--------|-----------|-------------|
| PF-001 | Receiver sensitivity | < -120 dBm MDS | [ ] | [ ] | Signal gen |
| PF-002 | Transmitter spurious emissions | < -43 dBc | [ ] | [ ] | Spectrum analyzer |
| PF-003 | Frequency stability (Si5351) | ±2.5 ppm | [ ] | [ ] | Frequency counter |
| PF-004 | PA efficiency (Class-E) | > 80% | [ ] | [ ] | Power measurement |
| PF-005 | Audio latency (DSP) | < 20 ms | [ ] | [ ] | Scope |

### Hardware Interface Requirements (HI)

| ID | Requirement | Priority | Status | Validated | Test Method |
|----|-------------|----------|--------|-----------|-------------|
| HI-001 | STM32G474RET6 MCU @ 170MHz | Must | [x] | [ ] | Boot test |
| HI-002 | I2C1 bus for Si5351A (0x60), BQ76920 (0x08), TPS25750D (0x21) | Must | [ ] | [ ] | I2C scan |
| HI-003 | USB-C #1: Power + MCU USB (TPS25750D controlled) | Must | [ ] | [ ] | Enumeration |
| HI-004 | USB-C #2: Debug via FT4232H (JTAG + dual UART) | Must | [ ] | [ ] | Debug session |
| HI-005 | 5 latching relays for LPF band selection | Must | [ ] | [ ] | Relay test |
| HI-006 | ADC for I/Q RX sampling, SWR measurement | Must | [ ] | [ ] | ADC reading |
| HI-007 | DAC for audio output | Must | [ ] | [ ] | Audio test |
| HI-008 | HRTIM for Class-E PA PWM drive | Must | [ ] | [ ] | Scope |

### Safety Requirements (SR)

| ID | Requirement | Priority | Status | Validated | Test Method |
|----|-------------|----------|--------|-----------|-------------|
| SR-001 | SWR > 3:1 reduces power automatically | Must | [ ] | [ ] | Antenna test |
| SR-002 | SWR > 5:1 disables TX, alerts user | Must | [ ] | [ ] | Antenna test |
| SR-003 | Thermal shutdown at PA > 85°C | Must | [ ] | [ ] | Thermal test |
| SR-004 | Battery undervoltage cutoff at 2.8V/cell | Must | [ ] | [ ] | Discharge test |
| SR-005 | Overcurrent protection at 10A discharge | Must | [ ] | [ ] | Load test |

---

## Schematic Implementation Status

### Main Sheet (sdr_transceiver.kicad_sch)

| Subsystem | Sheet Status | Hierarchical Labels | ERC Status |
|-----------|--------------|---------------------|------------|
| Power Management | [x] Placed | [x] Defined | [ ] Clean |
| MCU STM32G4 | [x] Placed | [x] Defined | [ ] Clean |
| Clock Si5351 | [x] Placed | [x] Defined | [ ] Clean |
| RF Section | [x] Placed | [x] Defined | [ ] Clean |
| Debug FT4232 | [x] Placed | [x] Defined | [ ] Clean |
| USB Connectors | [x] Placed | [x] Defined | [ ] Clean |

### ERC Error Summary (2025-12-25)

**Total Violations: 344**

Key issues to resolve:
1. `hier_label_mismatch`: Sheet pins without matching labels inside sheets (5+ instances)
2. `pin_not_connected`: Unconnected power pins (VBUS, etc.)
3. `unconnected_wire_endpoint`: Dangling wires (20+ instances)
4. `label_dangling`: Labels not connected to wires (10+ instances)
5. `wire_dangling`: Wire segments not connected to anything

### Missing Hierarchical Label Mappings

The following sheet pins on the main sheet are missing corresponding labels in sub-sheets:

| Sheet Pin | Expected In | Resolution |
|-----------|-------------|------------|
| TPS_INT | power_management.kicad_sch | Add hierarchical label |
| PWM_PA | mcu_stm32g4.kicad_sch | Add hierarchical label |
| BAND_SEL[0..4] | mcu_stm32g4.kicad_sch | Add bus hierarchical label |
| USB_DP | mcu_stm32g4.kicad_sch | Add hierarchical label |
| USB_DM | mcu_stm32g4.kicad_sch | Add hierarchical label |

---

## Firmware Implementation Status

### Module Status

| Module | Status | Tests | Notes |
|--------|--------|-------|-------|
| types.rs | [x] Complete | [x] | Frequency, Mode, Band, PowerLevel |
| dsp/filter.rs | [x] Complete | [x] | FIR filter with configurable taps |
| dsp/agc.rs | [x] Complete | [x] | Attack/decay/hang AGC |
| dsp/oscillator.rs | [x] Complete | [x] | NCO with phase accumulator |
| dsp/modulation.rs | [x] Complete | [x] | SSB/CW/AM/FM demod, IQ ops |
| hal/i2c.rs | [x] Complete | [ ] | Async I2C driver |
| hal/adc.rs | [x] Complete | [ ] | DMA-based ADC |
| hal/dac.rs | [x] Complete | [ ] | Audio output DAC |
| hal/pwm.rs | [x] Complete | [ ] | HRTIM PWM |
| hal/timer.rs | [x] Complete | [ ] | Timing utilities |
| hal/gpio.rs | [x] Complete | [ ] | GPIO control |
| dsp/filter_design.rs | [x] Complete | [x] | Biquad coefficient design, CW/SSB/AM presets |
| dsp/audio_chain.rs | [x] Complete | [x] | Audio processing pipeline, mode-specific chains |
| dsp/noise_reduction.rs | [x] Complete | [x] | Noise blanker, LMS filter, spectral NR |
| dsp/spectrum.rs | [x] Complete | [x] | Sliding DFT, waterfall display, peak detection |
| drivers/si5351.rs | [x] Complete | [x] | Clock synthesizer driver (via si5351_calc) |
| drivers/display.rs | [x] Complete | [ ] | SSD1306 OLED driver |
| drivers/encoder.rs | [x] Complete | [ ] | Rotary encoder input |
| radio/state.rs | [x] Complete | [x] | Radio state machine |
| radio/vfo.rs | [x] Complete | [x] | VFO frequency control |
| radio/transmit.rs | [x] Complete | [x] | TX sequencing |
| radio/keyer.rs | [x] Complete | [x] | Iambic keyer, Morse encoder |
| power.rs | [x] Complete | [ ] | Power management |
| protocol.rs | [x] Complete | [ ] | CAT command parser |
| usb/cdc.rs | [x] Complete | [ ] | USB CDC ACM |
| ui.rs | [x] Complete | [ ] | Menu and display |
| config.rs | [x] Complete | [ ] | System constants |
| main.rs | [x] Complete | [ ] | Entry point |

### Build Status (Updated 2025-12-26)

```
cargo check: PASS (0 warnings)
cargo test --features std: PASS (594 tests)
cargo build --release: PASS (embedded target, thumbv7em-none-eabihf)
```

---

## Test Plan

### Unit Tests (Host)

| Test File | Tests | Coverage | Status |
|-----------|-------|----------|--------|
| tests/types_tests.rs | 47 | Frequency, Band, Mode, PowerLevel, SWR, edges | [x] Pass |
| tests/dsp_tests.rs | 37 | Biquad, FIR, DC blocker, latency tests | [x] Pass |
| tests/agc_tests.rs | 28 | AGC, S-meter calibration | [x] Pass |
| tests/oscillator_tests.rs | 27 | NCO, sine, quadrature, frequency accuracy | [x] Pass |
| tests/modulation_tests.rs | 52 | IQ, AM, FM, SSB, Hilbert, roundtrip | [x] Pass |
| tests/protocol_tests.rs | 54 | CAT parser, response formatter | [x] Pass |
| tests/radio_tests.rs | 93 | VFO, RadioState, TxController, VOX | [x] Pass |
| lib.rs (si5351_calc) | 19 | Si5351 PLL/Multisynth, fractional-N | [x] Pass |
| lib.rs (keyer) | 20 | Iambic keyer, Morse encoder | [x] Pass |
| lib.rs (filter_design) | 18 | Biquad design, CW/SSB/AM filters | [x] Pass |
| lib.rs (audio_chain) | 23 | Audio chain, mode filters, notch | [x] Pass |
| lib.rs (noise_reduction) | 29 | NB, LMS, spectral NR, chain | [x] Pass |
| lib.rs (spectrum) | 21 | Sliding DFT, waterfall, peak detect | [x] Pass |
| tests/power_tests.rs | 31 | Battery, temp, power manager | [x] Pass |
| tests/config_tests.rs | 42 | Constants, pins, timers, DMA | [x] Pass |
| tests/encoder_tests.rs | 27 | Quadrature decoder, acceleration, bounds | [x] Pass |
| tests/buffer_tests.rs | 26 | Ring buffers, write buffers, CAT roundtrip | [x] Pass |
| **Total** | **594** | | **PASS** |

### Integration Tests (Target)

| Test | Hardware Required | Status |
|------|-------------------|--------|
| I2C scan | Dev board + Si5351 | [ ] Pending |
| USB enumeration | Dev board | [ ] Pending |
| ADC sampling | Dev board + signal source | [ ] Pending |
| Audio output | Dev board + speaker | [ ] Pending |
| RF TX | Full assembly + spectrum analyzer | [ ] Pending |
| RF RX | Full assembly + signal generator | [ ] Pending |

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-12-25 | Initial requirements document from design docs |
| 1.1 | 2025-12-25 | Added 123 unit tests, fixed all warnings, verified embedded build |
| 1.2 | 2025-12-25 | Added 68 more tests (protocol, SSB), total 191 tests passing |
| 1.3 | 2025-12-25 | Added 93 radio tests (VFO, state, TX controller, VOX), total 284 tests |
| 1.4 | 2025-12-26 | Added Si5351 fractional-N frequency calculator with 19 tests, total 303 tests |
| 1.5 | 2025-12-26 | Added CW keyer module (iambic A/B, bug, ultimatic, Morse encoder), 20 tests, total 323 |
| 1.6 | 2025-12-26 | Added filter_design module (biquad coefficients, CW/SSB/AM bandwidth presets), 18 tests, total 341 |
| 1.7 | 2025-12-26 | Added S-meter calibration tests (6dB/S-unit, dynamic range, clamping), 7 tests, total 348 |
| 1.8 | 2025-12-26 | Added audio_chain module (CW/SSB/AM/FM pipelines, notch filter), 23 tests, total 371 |
| 1.9 | 2025-12-26 | Added noise_reduction module (NB, LMS, spectral NR), 29 tests, total 400 |
| 2.0 | 2025-12-26 | Added spectrum module (sliding DFT, waterfall, peak detection), 21 tests, total 421 |
| 2.1 | 2025-12-26 | Added power management tests (battery, thermal, limits), 31 tests, total 452 |
| 2.2 | 2025-12-26 | Added config tests (constants, pins, timing), 42 tests, total 494. Binary: 14KB flash, 5.5KB RAM |
| 2.3 | 2025-12-26 | Added encoder tests (quadrature, acceleration, bounded values), 27 tests, total 521 |
| 2.4 | 2025-12-26 | Added FIR filter tests (coefficients, lowpass, bandpass, impulse response), 12 tests, total 533 |
| 2.5 | 2025-12-26 | Added buffer tests (ring buffer, write buffer, CAT roundtrip), 26 tests |
| 2.6 | 2025-12-26 | Added oscillator frequency accuracy tests (zero crossings, RMS, orthogonality), 7 tests, total 566 |
| 2.7 | 2025-12-26 | Added IqSample methods (normalize, scale, add, sub), Hilbert/SSB roundtrip tests, 9 tests, total 575 |
| 2.8 | 2025-12-26 | Added DSP latency tests (PF-005 validation), 4 tests, total 579 |
| 2.9 | 2025-12-26 | Added band edge, mode BFO/bandwidth, SWR threshold tests, 15 tests, total 594 |
