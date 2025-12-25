# Software Defined Radio Design Document

## Project: STM32-Based SDR Transceiver with USB-C Power Delivery

**Version:** 1.0  
**Date:** December 2024  
**Based on:** uSDX/truSDX H-Bridge Architecture

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [System Overview](#2-system-overview)
3. [Requirements](#3-requirements)
4. [Hardware Architecture](#4-hardware-architecture)
5. [Power System Design](#5-power-system-design)
6. [RF Signal Chain](#6-rf-signal-chain)
7. [Digital Subsystem](#7-digital-subsystem)
8. [Concept of Operations](#8-concept-of-operations)
9. [Bill of Materials](#9-bill-of-materials)
10. [Design Tradeoffs](#10-design-tradeoffs)
11. [Firmware Architecture](#11-firmware-architecture)
12. [Test Plan](#12-test-plan)

---

## 1. Executive Summary

This document describes a software-defined radio (SDR) transceiver design based on the uSDX/truSDX H-bridge Class-E power amplifier architecture, modernized with:

- **STM32G4 microcontroller** with integrated USB Power Delivery (UCPD)
- **Dual USB-C connectors**: one for power/data, one for debug/JTAG
- **3S lithium battery** with PMBus-compatible charging and monitoring
- **Flexible LPF switching** supporting both latching relays and MOSFET options

The design targets HF amateur radio bands (80m-10m) with QRP power levels (0.5-5W), supporting CW, SSB, and digital modes.

---

## 2. System Overview

### 2.1 Block Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SDR TRANSCEIVER SYSTEM                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐     ┌──────────────────────────────────────────────────┐ │
│  │  USB-C #1    │     │              POWER MANAGEMENT                    │ │
│  │  (Power+Data)│────►│  TCPP03-M20 ──► STM32G4 UCPD ──► BQ25713        │ │
│  │  Up to 20V   │     │                                    │             │ │
│  └──────────────┘     │                                    ▼             │ │
│                       │                              3S Li-Ion Pack      │ │
│                       │                              (9.6-12.6V)         │ │
│                       │                                    │             │ │
│                       │                              BQ76920 AFE         │ │
│                       │                              (Fuel Gauge)        │ │
│                       └──────────────────────────────────────────────────┘ │
│                                         │                                   │
│                                         ▼                                   │
│  ┌──────────────┐     ┌──────────────────────────────────────────────────┐ │
│  │  USB-C #2    │     │              DIGITAL SUBSYSTEM                   │ │
│  │  (Debug)     │────►│  FT4232H ◄──► STM32G474 ◄──► Si5351A            │ │
│  │  JTAG+Serial │     │  (JTAG/UART)  (Main MCU)     (Clock Synth)       │ │
│  └──────────────┘     └──────────────────────────────────────────────────┘ │
│                                         │                                   │
│                                         ▼                                   │
│                       ┌──────────────────────────────────────────────────┐ │
│                       │              RF SUBSYSTEM                         │ │
│                       │                                                   │ │
│                       │  ┌─────────┐   ┌─────────┐   ┌─────────────────┐ │ │
│                       │  │Quadrature│   │ H-Bridge│   │  LPF Bank       │ │ │
│                       │  │Sampling │◄─►│ PA      │──►│  (5 bands)      │ │ │
│                       │  │Detector │   │ uP9636  │   │  Relay/MOSFET   │ │ │
│                       │  └─────────┘   └─────────┘   └────────┬────────┘ │ │
│                       │       │                               │          │ │
│                       │       │         ┌─────────────────────┘          │ │
│                       │       ▼         ▼                                │ │
│                       │  ┌─────────────────────┐                         │ │
│                       │  │    SWR Bridge       │──────► ANT              │ │
│                       │  │  (Power/SWR Meter)  │                         │ │
│                       │  └─────────────────────┘                         │ │
│                       └──────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Key Features

| Feature | Specification |
|---------|---------------|
| Frequency Range | 3.5 - 30 MHz (80m - 10m) |
| Output Power | 0.5 - 5W (adjustable) |
| Modes | CW, LSB, USB, AM, FM, Digital |
| Power Input | USB-C PD (5-20V), 3S Li-Ion |
| Battery | 3S 18650 (9.6-12.6V, ~3000mAh) |
| Current Draw | RX: ~100mA, TX: ~500mA @ 5W |
| Debug Interface | JTAG + Dual Serial via FT4232H |

---

## 3. Requirements

### 3.1 Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-001 | Shall transmit and receive on 80m, 40m, 30m, 20m, and 10m bands | Must |
| FR-002 | Shall support CW, SSB (LSB/USB), and digital modes | Must |
| FR-003 | Shall provide adjustable output power from 0.5W to 5W | Must |
| FR-004 | Shall charge internal battery from USB-C Power Delivery | Must |
| FR-005 | Shall operate from battery when USB power unavailable | Must |
| FR-006 | Shall display SWR and forward/reflected power | Should |
| FR-007 | Shall support CAT control via USB serial | Should |
| FR-008 | Shall provide JTAG debug access via dedicated USB-C port | Must |

### 3.2 Power Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| PR-001 | Shall negotiate USB PD up to 20V/3A (60W max) | Must |
| PR-002 | Shall charge 3S Li-Ion pack with CC/CV profile | Must |
| PR-003 | Shall provide battery protection (OVP, UVP, OCP, OTP) | Must |
| PR-004 | Shall report battery state via I2C/SMBus | Should |
| PR-005 | Shall support dead battery boot from USB-C | Must |
| PR-006 | Shall provide OTG capability (5V output when on battery) | Could |

### 3.3 Performance Requirements

| ID | Requirement | Value |
|----|-------------|-------|
| PF-001 | Receiver sensitivity | < -120 dBm MDS |
| PF-002 | Transmitter spurious emissions | < -43 dBc |
| PF-003 | Frequency stability | ±2.5 ppm |
| PF-004 | PA efficiency | > 80% (Class E) |
| PF-005 | Battery charge time (0-80%) | < 2 hours |

---

## 4. Hardware Architecture

### 4.1 Microcontroller Selection

**Selected: STM32G474RET6**

| Feature | Specification | Use Case |
|---------|---------------|----------|
| Core | ARM Cortex-M4F @ 170 MHz | DSP for audio/signal processing |
| Flash | 512 KB | Firmware + configuration storage |
| RAM | 128 KB | Audio buffers, FFT processing |
| USB | Full-Speed + UCPD | Data interface + PD negotiation |
| ADC | 5x 12-bit, 4 MSPS | Audio ADC, SWR measurement |
| DAC | 2x 12-bit | Audio output, PWM reference |
| Timers | 17 total, HRTIM | PWM generation for PA |
| I2C | 4 channels | Si5351, BQ25713, BQ76920, OLED |
| SPI | 4 channels | External flash, optional codec |
| UCPD | 2 ports | USB-C Power Delivery |

**Rationale:** The STM32G4 series provides integrated UCPD eliminating external PD controller, HRTIM for precise PWM generation, and sufficient DSP capability for audio processing.

### 4.2 USB-C Port Architecture

#### Port 1: Power + MCU USB (Primary)

```
USB-C Connector #1
        │
        ├── CC1/CC2 ──► TCPP03-M20 ──► STM32G4 UCPD1
        │                   │
        │                   ├── VBUS Control (N-FET gate drive)
        │                   ├── OVP/OCP Protection
        │                   └── Dead Battery Support
        │
        ├── VBUS ──► BQ25713 (Charger Input)
        │                │
        │                └──► System Power Rail
        │
        ├── D+/D- ──► STM32G4 USB-FS
        │              (CDC-ACM for CAT control)
        │
        └── SBU1/SBU2 ── (Reserved for future use)
```

#### Port 2: Debug Interface

```
USB-C Connector #2
        │
        ├── VBUS ──► 5V input (self-powered FT4232H)
        │
        └── D+/D- ──► FT4232H
                        ├── Channel A: JTAG TCK
                        ├── Channel B: JTAG TMS/TDI/TDO
                        ├── Channel C: UART (Console/Debug)
                        └── Channel D: UART (CAT backup/Aux)
```

### 4.3 Clock Generation

**Si5351A-B-GT** (I2C programmable clock synthesizer)

| Output | Function | Frequency Range |
|--------|----------|-----------------|
| CLK0 | Quadrature I clock | 3.5 - 30 MHz |
| CLK1 | Quadrature Q clock (90° shift) | 3.5 - 30 MHz |
| CLK2 | TX carrier / BFO | 3.5 - 30 MHz |

The Si5351 uses a 25/27 MHz crystal and dual PLLs to generate all required frequencies with sub-Hz resolution.

---

## 5. Power System Design

### 5.1 Power Architecture

```
                                    ┌─────────────────────────────────┐
USB-C VBUS ─────────────────────────┤                                 │
(5-20V from PD)                     │         BQ25713B                │
        │                           │    Buck-Boost Charger           │
        │                           │                                 │
        │   ┌───────────────────────┤ VBUS ────────────────► VSYS     ├──► 12V Rail
        │   │                       │                        (9-14V)  │
        │   │                       │ BAT ─────────┐                  │
        │   │                       │              │                  │
        │   │                       │ I2C ◄────────┼──────────────────┤
        │   │                       └──────────────┼──────────────────┘
        │   │                                      │
        │   │                                      ▼
        │   │                           ┌─────────────────────┐
        │   │                           │    3S Li-Ion Pack   │
        │   │                           │   (9.6V - 12.6V)    │
        │   │                           │                     │
        │   │                           │  Cell 1 ─┬─ 3.0-4.2V│
        │   │                           │  Cell 2 ─┼─ 3.0-4.2V│
        │   │                           │  Cell 3 ─┴─ 3.0-4.2V│
        │   │                           └─────────┬───────────┘
        │   │                                     │
        │   │                                     ▼
        │   │                           ┌─────────────────────┐
        │   │                           │     BQ76920         │
        │   │                           │  Battery AFE        │
        │   │                           │                     │
        │   │                           │ • Cell balancing    │
        │   │                           │ • Voltage monitor   │
        │   │                           │ • Coulomb counter   │
        │   │                           │ • Temp sensing      │
        │   │                           │ • I2C telemetry     │
        │   │                           └─────────────────────┘
        │   │
        ▼   ▼
┌───────────────────────────────────────────────────────────────────┐
│                        POWER DISTRIBUTION                         │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│   VSYS (9-14V) ──┬──► H-Bridge PA (uP9636)                       │
│                  │                                                │
│                  ├──► 5V Buck ──┬──► FT4232H                     │
│                  │   (TPS62160) │                                 │
│                  │              ├──► Si5351A (VDDO)               │
│                  │              │                                 │
│                  │              └──► USB VBUS (OTG mode)          │
│                  │                                                │
│                  └──► 3.3V LDO ──┬──► STM32G4                     │
│                      (AMS1117)   │                                │
│                                  ├──► Si5351A (VDD core)          │
│                                  │                                │
│                                  └──► Misc analog                 │
└───────────────────────────────────────────────────────────────────┘
```

### 5.2 USB Power Delivery Negotiation

The STM32G4's integrated UCPD peripheral handles PD negotiation:

**Supported Power Profiles (Sink):**

| PDO | Voltage | Current | Power |
|-----|---------|---------|-------|
| 1 | 5V | 3A | 15W |
| 2 | 9V | 3A | 27W |
| 3 | 12V | 3A | 36W |
| 4 | 15V | 3A | 45W |
| 5 | 20V | 3A | 60W |

**Charging Strategy:**

1. At power connection, UCPD negotiates highest available voltage
2. BQ25713 configured via I2C for 3S charging (12.6V, CC/CV)
3. Charge current limited based on negotiated power capability
4. System can run from VBUS directly (NVDC topology) while charging

### 5.3 Battery Management

**BQ76920 Configuration:**

| Parameter | Setting |
|-----------|---------|
| Cell Count | 3S |
| Overvoltage | 4.25V/cell |
| Undervoltage | 2.8V/cell |
| Overcurrent (Discharge) | 10A |
| Overcurrent (Charge) | 3A |
| Short Circuit | 50A, 200µs |
| Temperature Range | 0-45°C (charge), -20-60°C (discharge) |

**Telemetry (I2C readable):**

- Individual cell voltages
- Pack voltage and current
- State of charge (Coulomb counting)
- Temperature (NTC)
- Fault status flags

---

## 6. RF Signal Chain

### 6.1 Receiver Path

```
ANT ──► T/R Switch ──► BPF ──► LNA ──► Quadrature Sampling Detector ──► Audio
            │                              │
            │                    ┌─────────┴─────────┐
            │                    │                   │
            │               I Channel           Q Channel
            │                    │                   │
            │              LPF (3kHz)          LPF (3kHz)
            │                    │                   │
            │              ADC (STM32)         ADC (STM32)
            │                    │                   │
            │                    └─────────┬─────────┘
            │                              │
            │                         DSP (Hilbert)
            │                              │
            │                         Audio Out
            │
            └── (TX path below)
```

**Quadrature Sampling Detector:**

The QSD uses the Si5351's CLK0/CLK1 outputs (0° and 90° phase) to directly sample the RF signal to baseband. This is a Tayloe detector topology using FST3253 analog multiplexers.

### 6.2 Transmitter Path

```
Audio In ──► ADC ──► DSP ──► PWM Generation ──► H-Bridge PA ──► LPF ──► ANT
                              (HRTIM)           (uP9636)
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Class E Operation    │
                    │                       │
                    │  • Square wave drive  │
                    │  • ZVS switching      │
                    │  • 80%+ efficiency    │
                    └───────────────────────┘
```

### 6.3 H-Bridge Power Amplifier

**uP9636 Configuration:**

| Parameter | Value |
|-----------|-------|
| VIN | 9-14V (from battery/VSYS) |
| VCC | 12V (gate driver supply) |
| Max Current | 60A per MOSFET (continuous) |
| Rds(on) | 5.5mΩ typical |
| Switching Freq | 3.5-30 MHz (RF frequency) |

The H-bridge is driven directly at the RF carrier frequency. The Class E output network shapes the square wave into a sinusoid while recovering reactive power.

### 6.4 Low Pass Filter Bank

**5-Band LPF Configuration:**

| Band | Frequency | LPF Cutoff | Relay |
|------|-----------|------------|-------|
| 80m | 3.5-4.0 MHz | 4.5 MHz | K1 |
| 40m | 7.0-7.3 MHz | 8 MHz | K2 |
| 30m/20m | 10.1-14.35 MHz | 15 MHz | K3 |
| 17m/15m | 18.1-21.45 MHz | 22 MHz | K4 |
| 12m/10m | 24.9-29.7 MHz | 30 MHz | K5 |

### 6.5 LPF Switching: Latching Relay vs MOSFET Analysis

#### Option A: Latching Relays (Recommended)

**Advantages:**
- Zero quiescent current (only pulse to switch)
- Excellent RF isolation when open (~40+ dB at HF)
- State retention through power cycles
- Very low insertion loss (<0.1 dB)
- Handles full TX power without dissipation concerns
- Simple drive circuit (pulse from GPIO via transistor)

**Disadvantages:**
- Larger physical size
- Mechanical wear (100k-1M cycles typical)
- Slower switching (~5-10ms)
- Higher cost per relay

**Recommended Parts:**
- EC2-3SNU (Kemet) - SMD, DPDT, 3V coil
- G6JU-2P-Y (Omron) - Through-hole, DPDT, 5V coil

**Drive Circuit:**
```
GPIO ──► 2N7002 ──┬──► Relay SET coil ──► GND
                  │
GPIO ──► 2N7002 ──┴──► Relay RESET coil ──► GND
                  │
                  └──► 100µF (shared pulse cap)
```

#### Option B: MOSFET Switches

**Advantages:**
- No mechanical wear (unlimited cycles)
- Very fast switching (<1µs)
- Smaller footprint
- Lower cost

**Disadvantages:**
- Rds(on) causes insertion loss and heating
- Parasitic capacitance couples RF when "off"
- Requires continuous gate drive (power consumption)
- Body diode can conduct on negative RF swings
- Higher voltage MOSFETs have worse Rds(on)

**If using MOSFETs:**
- Use back-to-back N-FETs for bidirectional blocking
- Select parts with low Coss (<50pF) for good isolation
- Consider RF-specific analog switches (PE42420, ADG901)

#### Recommendation

**For battery-powered operation at >5V: Use latching relays**

Rationale:
1. Zero standby power aligns with battery optimization
2. State retention means band selection survives sleep/wake
3. Clean RF path without FET parasitics
4. No thermal concerns at 5W output
5. Switching happens infrequently (band changes), so cycle life is not a concern

---

## 7. Digital Subsystem

### 7.1 I2C Bus Architecture

```
STM32G4 I2C1 (400 kHz)
        │
        ├──► Si5351A (0x60) - Clock synthesizer
        │
        ├──► BQ25713 (0x6B) - Battery charger
        │
        ├──► BQ76920 (0x08) - Battery AFE
        │
        ├──► TCPP03-M20 (0x35) - USB-C port protection
        │
        └──► SSD1306 (0x3C) - OLED display (optional)
```

### 7.2 GPIO Allocation

| Function | STM32 Pin | Direction | Notes |
|----------|-----------|-----------|-------|
| PTT | PA0 | Input | Active low, external pullup |
| Band Relay SET | PA1-PA5 | Output | Active high pulse |
| Band Relay RESET | PA6-PA10 | Output | Active high pulse |
| TX Enable | PB0 | Output | Enables PA bias |
| Si5351 CLK_EN | PB1 | Output | Clock output enable |
| SWR FWD ADC | PC0 | Analog | Forward power sense |
| SWR REV ADC | PC1 | Analog | Reflected power sense |
| Audio In ADC | PC2 | Analog | Microphone input |
| Audio Out DAC | PA4 | Analog | Speaker/headphone |
| I2C1 SCL | PB6 | AF | I2C clock |
| I2C1 SDA | PB7 | AF | I2C data |
| UCPD1 CC1 | PB4 | AF | USB-C CC line 1 |
| UCPD1 CC2 | PB5 | AF | USB-C CC line 2 |
| USB DP | PA12 | AF | USB data + |
| USB DM | PA11 | AF | USB data - |

### 7.3 FT4232H Configuration

| Channel | Mode | Function | Baud Rate |
|---------|------|----------|-----------|
| A | MPSSE | JTAG TCK/TMS | N/A |
| B | MPSSE | JTAG TDI/TDO | N/A |
| C | UART | Debug console | 115200 |
| D | UART | CAT control (backup) | 9600-115200 |

EEPROM configured for:
- Self-powered mode (from USB-C #2 VBUS)
- Separate VIO domains
- FT_PROG configured channel modes

---

## 8. Concept of Operations

### 8.1 Power-On Sequence

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         POWER-ON SEQUENCE                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  1. USB-C Connected OR Battery Present                                  │
│     │                                                                   │
│     ▼                                                                   │
│  2. TCPP03 enables VBUS path (if USB) or battery path                  │
│     │                                                                   │
│     ▼                                                                   │
│  3. 3.3V LDO enables → STM32G4 boots                                   │
│     │                                                                   │
│     ▼                                                                   │
│  4. STM32 initializes UCPD peripheral                                  │
│     │                                                                   │
│     ├── If USB connected: Negotiate PD contract                        │
│     │   │                                                               │
│     │   ▼                                                               │
│     │   Configure BQ25713 for negotiated voltage                       │
│     │   │                                                               │
│     │   ▼                                                               │
│     │   Begin charging if battery < 12.6V                              │
│     │                                                                   │
│     └── If battery only: Read BQ76920 for SOC                          │
│         │                                                               │
│         ▼                                                               │
│         If SOC < 10%: Display low battery warning                      │
│     │                                                                   │
│     ▼                                                                   │
│  5. Initialize Si5351A clock synthesizer                               │
│     │                                                                   │
│     ▼                                                                   │
│  6. Load last-used frequency and mode from flash                       │
│     │                                                                   │
│     ▼                                                                   │
│  7. Select appropriate LPF (pulse latching relay)                      │
│     │                                                                   │
│     ▼                                                                   │
│  8. Enable receiver, begin audio processing                            │
│     │                                                                   │
│     ▼                                                                   │
│  9. OPERATIONAL - Ready for RX/TX                                      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Receive Operation

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         RECEIVE OPERATION                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  RF Signal at Antenna                                                   │
│     │                                                                   │
│     ▼                                                                   │
│  T/R Switch (in RX position)                                           │
│     │                                                                   │
│     ▼                                                                   │
│  Band-Pass Filter (selected for current band)                          │
│     │                                                                   │
│     ▼                                                                   │
│  Quadrature Sampling Detector                                          │
│     │                                                                   │
│     ├──► I Channel ──► Anti-alias LPF ──► STM32 ADC1                   │
│     │                                                                   │
│     └──► Q Channel ──► Anti-alias LPF ──► STM32 ADC2                   │
│                                           │                             │
│                                           ▼                             │
│                              ┌─────────────────────────┐               │
│                              │    DSP Processing       │               │
│                              │                         │               │
│                              │  • Hilbert transform    │               │
│                              │  • SSB demodulation     │               │
│                              │  • CW filtering         │               │
│                              │  • AGC                  │               │
│                              │  • Noise reduction      │               │
│                              └───────────┬─────────────┘               │
│                                          │                              │
│                                          ▼                              │
│                              DAC ──► Audio Amplifier ──► Speaker        │
│                                                                         │
│  Concurrent Tasks:                                                      │
│  • S-meter calculation from AGC level                                  │
│  • Spectrum display update (optional)                                  │
│  • Frequency display update                                            │
│  • Battery SOC monitoring                                              │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.3 Transmit Operation

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         TRANSMIT OPERATION                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  PTT Asserted (hardware interrupt)                                     │
│     │                                                                   │
│     ▼                                                                   │
│  TX Sequence Start                                                      │
│     │                                                                   │
│     ├─► 1. Disable receiver audio output (mute)                        │
│     │                                                                   │
│     ├─► 2. Switch T/R relay to TX (if not using QSK)                   │
│     │                                                                   │
│     ├─► 3. Configure Si5351 for TX frequency                           │
│     │      (may include RIT/XIT offset)                                │
│     │                                                                   │
│     ├─► 4. Enable PA bias (soft start)                                 │
│     │                                                                   │
│     ├─► 5. Begin audio sampling and modulation                         │
│     │                                                                   │
│     └─► 6. Monitor SWR continuously                                    │
│            │                                                            │
│            ├── If SWR > 3:1: Reduce power                              │
│            └── If SWR > 5:1: Disable TX, alert user                    │
│                                                                         │
│  During TX:                                                             │
│     │                                                                   │
│     ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    MODULATION MODES                              │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │                                                                  │   │
│  │  CW Mode:                                                        │   │
│  │    Key input ──► PWM gate ──► H-Bridge ──► LPF ──► ANT          │   │
│  │    (shaped envelope for click-free keying)                       │   │
│  │                                                                  │   │
│  │  SSB Mode:                                                       │   │
│  │    Mic ──► ADC ──► DSP (Hilbert/Weaver) ──► PWM ──► H-Bridge    │   │
│  │    (amplitude modulation of Class E PA)                          │   │
│  │                                                                  │   │
│  │  Digital Mode:                                                   │   │
│  │    USB Audio ──► ADC ──► PWM ──► H-Bridge ──► LPF ──► ANT       │   │
│  │    (constant carrier, audio modulation)                          │   │
│  │                                                                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  PTT Released                                                           │
│     │                                                                   │
│     ▼                                                                   │
│  TX Sequence End                                                        │
│     │                                                                   │
│     ├─► 1. Ramp down PA (soft stop)                                    │
│     │                                                                   │
│     ├─► 2. Disable PA bias                                             │
│     │                                                                   │
│     ├─► 3. Switch T/R relay to RX                                      │
│     │                                                                   │
│     ├─► 4. Restore RX frequency (remove RIT/XIT)                       │
│     │                                                                   │
│     └─► 5. Re-enable receiver audio                                    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.4 Band Change Operation

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         BAND CHANGE SEQUENCE                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  User requests band change (encoder/button/CAT)                        │
│     │                                                                   │
│     ▼                                                                   │
│  1. Verify not in TX mode (abort if PTT active)                        │
│     │                                                                   │
│     ▼                                                                   │
│  2. Save current frequency to band memory                              │
│     │                                                                   │
│     ▼                                                                   │
│  3. Mute audio output                                                  │
│     │                                                                   │
│     ▼                                                                   │
│  4. Determine new LPF required                                         │
│     │                                                                   │
│     ├── If same LPF: Skip to step 7                                    │
│     │                                                                   │
│     └── If different LPF:                                              │
│         │                                                               │
│         ▼                                                               │
│  5. Pulse RESET on current relay (opens current LPF)                   │
│     │                                                                   │
│     ▼                                                                   │
│  6. Pulse SET on new relay (closes new LPF)                            │
│     │                                                                   │
│     ▼                                                                   │
│  7. Update Si5351 to new frequency                                     │
│     │                                                                   │
│     ▼                                                                   │
│  8. Load saved frequency for new band (or band edge)                   │
│     │                                                                   │
│     ▼                                                                   │
│  9. Update display                                                     │
│     │                                                                   │
│     ▼                                                                   │
│  10. Un-mute audio                                                     │
│     │                                                                   │
│     ▼                                                                   │
│  Complete - Now receiving on new band                                  │
│                                                                         │
│  Timing: ~20ms total (dominated by relay settling)                     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.5 USB-C Power Events

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     USB-C POWER EVENT HANDLING                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  EVENT: USB-C Cable Connected                                          │
│     │                                                                   │
│     ▼                                                                   │
│  UCPD peripheral detects CC connection                                 │
│     │                                                                   │
│     ▼                                                                   │
│  Send Source_Capabilities request                                      │
│     │                                                                   │
│     ▼                                                                   │
│  Parse advertised PDOs                                                 │
│     │                                                                   │
│     ▼                                                                   │
│  Select optimal PDO (prefer highest voltage ≤20V)                      │
│     │                                                                   │
│     ▼                                                                   │
│  Send Request message                                                  │
│     │                                                                   │
│     ▼                                                                   │
│  Wait for Accept → PS_RDY                                              │
│     │                                                                   │
│     ▼                                                                   │
│  Configure BQ25713:                                                    │
│     • Set input voltage limit = negotiated - 0.5V                      │
│     • Set charge current based on power budget                         │
│     • Enable charging if battery needs it                              │
│     │                                                                   │
│     ▼                                                                   │
│  Update UI with power status                                           │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  EVENT: USB-C Cable Disconnected                                       │
│     │                                                                   │
│     ▼                                                                   │
│  UCPD peripheral detects CC disconnect                                 │
│     │                                                                   │
│     ▼                                                                   │
│  BQ25713 automatically switches to battery power (NVDC)                │
│     │                                                                   │
│     ▼                                                                   │
│  Check battery SOC via BQ76920                                         │
│     │                                                                   │
│     ├── If SOC > 20%: Continue normal operation                        │
│     │                                                                   │
│     └── If SOC < 20%: Display battery warning                          │
│         │                                                               │
│         └── If SOC < 5%: Initiate graceful shutdown                    │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  EVENT: Hard Reset / Power Source Change                               │
│     │                                                                   │
│     ▼                                                                   │
│  If in TX: Immediately terminate transmission                          │
│     │                                                                   │
│     ▼                                                                   │
│  Battery takes over seamlessly (NVDC topology)                         │
│     │                                                                   │
│     ▼                                                                   │
│  Re-negotiate PD contract when new source available                    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.6 Low Power / Sleep Mode

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         LOW POWER OPERATION                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Trigger: No user activity for configurable timeout (default 5 min)    │
│                                                                         │
│  SLEEP ENTRY:                                                           │
│     │                                                                   │
│     ├─► Disable Si5351 clock outputs                                   │
│     │                                                                   │
│     ├─► Power down audio amplifier                                     │
│     │                                                                   │
│     ├─► Disable OLED (or reduce brightness)                            │
│     │                                                                   │
│     ├─► STM32 enters STOP2 mode                                        │
│     │                                                                   │
│     └─► BQ25713 continues charging autonomously                        │
│                                                                         │
│  Wake Sources:                                                          │
│     • PTT input (GPIO EXTI)                                            │
│     • Encoder rotation                                                  │
│     • USB activity (UCPD event)                                        │
│     • RTC alarm (periodic battery check)                               │
│                                                                         │
│  SLEEP CURRENT BUDGET:                                                  │
│     │                                                                   │
│     ├── STM32 STOP2: ~2 µA                                             │
│     ├── BQ25713 standby: ~8 µA                                         │
│     ├── BQ76920 ship mode: ~2 µA                                       │
│     ├── Si5351 disabled: ~1 µA                                         │
│     ├── TCPP03 standby: ~5 µA                                          │
│     └── Relay leakage: ~0 µA (latching, no holding current)            │
│         ─────────────────────                                          │
│         Total: ~18 µA                                                   │
│                                                                         │
│  Battery Life (sleep): 3000mAh / 0.018mA = ~19 years (theoretical)     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 9. Bill of Materials

### 9.1 Major Components

| Ref | Part Number | Description | Qty | Est. Cost |
|-----|-------------|-------------|-----|-----------|
| U1 | STM32G474RET6 | MCU, ARM Cortex-M4, 512K Flash | 1 | $8.50 |
| U2 | Si5351A-B-GT | Clock synthesizer, I2C, 8 outputs | 1 | $2.50 |
| U3 | BQ25713RSNR | Buck-boost charger, 1-4S, I2C | 1 | $4.20 |
| U4 | BQ76920PW | Battery AFE, 3-5S, I2C | 1 | $2.80 |
| U5 | TCPP03-M20 | USB-C port protection, DRP | 1 | $1.50 |
| U6 | FT4232HL | Quad USB-UART/JTAG | 1 | $5.50 |
| U7 | uP9636 | H-bridge driver w/ MOSFETs | 1 | $3.00 |
| U8 | TPS62160 | 3A buck converter, 3-17V in | 1 | $1.80 |
| U9 | AMS1117-3.3 | 1A LDO, 3.3V | 1 | $0.30 |
| J1 | USB-C Receptacle | 16-pin, mid-mount | 2 | $1.00 |
| K1-K5 | EC2-3SNU | Latching relay, DPDT, 3V | 5 | $4.00 |
| Y1 | 25MHz Crystal | 18pF, ±10ppm, for Si5351 | 1 | $0.50 |
| Y2 | 8MHz Crystal | 20pF, ±10ppm, for STM32 | 1 | $0.40 |

### 9.2 Passive Components (Summary)

| Category | Count | Est. Cost |
|----------|-------|-----------|
| Resistors (0402/0603) | ~80 | $2.00 |
| Capacitors (MLCC) | ~60 | $3.00 |
| Inductors (power) | 6 | $4.00 |
| Inductors (RF, toroids) | 15 | $8.00 |
| Ferrite beads | 8 | $1.00 |

### 9.3 Cost Summary

| Category | Cost |
|----------|------|
| ICs and Semiconductors | $35.00 |
| Passives | $18.00 |
| Connectors | $5.00 |
| PCB (4-layer, 100x80mm) | $15.00 |
| Enclosure | $10.00 |
| Battery (3S 18650) | $15.00 |
| **Total BOM** | **~$98** |

---

## 10. Design Tradeoffs

### 10.1 Latching Relay vs MOSFET for LPF Switching

| Factor | Latching Relay | MOSFET Switch | Winner |
|--------|----------------|---------------|--------|
| Quiescent Current | 0 µA | 10-100 µA | Relay |
| RF Isolation (off) | >40 dB | 20-30 dB | Relay |
| Insertion Loss (on) | <0.1 dB | 0.2-0.5 dB | Relay |
| Size | 10x6x5mm | 3x3mm | MOSFET |
| Cost (per switch) | $0.80 | $0.30 | MOSFET |
| Lifetime | 100k cycles | Unlimited | MOSFET |
| Power Handling | Excellent | Good (thermal) | Relay |
| Voltage Tolerance | Easy | Needs selection | Relay |

**Decision: Latching Relays**

For a battery-powered HF transceiver operating above 5V, latching relays provide superior performance where it matters most: zero standby power, excellent RF characteristics, and robust operation.

### 10.2 Integrated UCPD vs External PD Controller

| Factor | STM32 UCPD | External (FUSB302, etc.) | Winner |
|--------|------------|--------------------------|--------|
| BOM Cost | $0 (included) | $2-4 | STM32 |
| Board Space | Minimal | 4x4mm QFN + passives | STM32 |
| Firmware Complexity | Higher | Lower (I2C config) | External |
| Flexibility | Full control | Limited by chip | STM32 |
| USB-IF Certification | ST-provided stack | Chip vendor stack | Tie |

**Decision: STM32 Integrated UCPD**

The STM32G4's integrated UCPD eliminates an external chip and provides full control over PD negotiation. ST's X-CUBE-TCPP software package is USB-IF certified.

### 10.3 Single vs Dual USB-C Connectors

| Factor | Single USB-C | Dual USB-C |
|--------|--------------|------------|
| Cost | Lower | +$1.50 |
| Complexity | Lower | Higher |
| Debug Access | Shared with power | Dedicated |
| Simultaneous charge + debug | Possible but complex | Easy |
| Field updates | Requires USB | JTAG backup available |

**Decision: Dual USB-C**

Separating power/data from debug provides cleaner design, easier development, and doesn't require special cables or hubs for simultaneous debugging and powered operation.

---

## 11. Firmware Architecture

### 11.1 Software Stack

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         APPLICATION LAYER                               │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ │
│  │   Radio   │ │   Power   │ │    UI     │ │    CAT    │ │  Config   │ │
│  │  Control  │ │  Manager  │ │  Manager  │ │  Handler  │ │  Storage  │ │
│  └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ │
│        │             │             │             │             │        │
├────────┴─────────────┴─────────────┴─────────────┴─────────────┴────────┤
│                         MIDDLEWARE LAYER                                │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐               │
│  │  USB-PD   │ │   Audio   │ │   DSP     │ │  FreeRTOS │               │
│  │   Stack   │ │  Codec    │ │  Library  │ │   CMSIS   │               │
│  │ (X-CUBE)  │ │           │ │  (CMSIS)  │ │           │               │
│  └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘               │
│        │             │             │             │                      │
├────────┴─────────────┴─────────────┴─────────────┴──────────────────────┤
│                           HAL LAYER                                     │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐      │
│  │ ADC │ │ DAC │ │HRTIM│ │ I2C │ │ SPI │ │ USB │ │UCPD │ │GPIO │      │
│  └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘      │
│     │       │       │       │       │       │       │       │          │
├─────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴──────────┤
│                         HARDWARE                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

### 11.2 RTOS Task Structure

| Task | Priority | Stack | Period | Function |
|------|----------|-------|--------|----------|
| DSP_Audio | Highest | 2KB | 48kHz interrupt | Audio sample processing |
| Radio_Control | High | 1KB | Event-driven | TX/RX state machine |
| Power_Manager | High | 1KB | 100ms | Battery/charging monitor |
| USB_PD | Medium | 2KB | Event-driven | PD negotiation |
| UI_Update | Low | 1KB | 50ms | Display refresh |
| CAT_Handler | Low | 1KB | Event-driven | Serial command processing |
| Idle | Lowest | 256B | N/A | Sleep mode entry |

### 11.3 Key Algorithms

**Hilbert Transform (90° Phase Shift):**
- 31-tap FIR filter for I/Q generation
- Used for SSB modulation/demodulation
- CMSIS-DSP `arm_fir_f32()` optimized for Cortex-M4

**AGC (Automatic Gain Control):**
- Attack time: 1ms
- Decay time: 500ms
- Peak detector with exponential averaging

**SWR Calculation:**
```c
// Forward and reflected voltage from ADC
float gamma = v_reflected / v_forward;
float swr = (1 + gamma) / (1 - gamma);
```

---

## 12. Test Plan

### 12.1 Unit Tests

| Test ID | Component | Test Description | Pass Criteria |
|---------|-----------|------------------|---------------|
| UT-001 | Si5351 | Frequency accuracy | ±1 Hz at 14.2 MHz |
| UT-002 | Si5351 | Phase accuracy (I/Q) | 90° ±2° |
| UT-003 | BQ25713 | I2C communication | Read/write all registers |
| UT-004 | BQ25713 | Charge profile | CC/CV within 1% |
| UT-005 | UCPD | PD negotiation | Contract at 20V/3A |
| UT-006 | LPF Relay | Switching | <10ms, reliable |
| UT-007 | H-Bridge | PWM output | 1-30 MHz, clean edges |

### 12.2 Integration Tests

| Test ID | Subsystem | Test Description | Pass Criteria |
|---------|-----------|------------------|---------------|
| IT-001 | RX Chain | Sensitivity test | MDS < -120 dBm |
| IT-002 | TX Chain | Power output | 5W ±0.5 dB |
| IT-003 | TX Chain | Spurious emissions | < -43 dBc |
| IT-004 | Power | Charge while TX | No brownout |
| IT-005 | Power | Battery failover | Seamless switch |
| IT-006 | System | Band change | <50ms, no glitch |

### 12.3 Environmental Tests

| Test ID | Condition | Test Description | Pass Criteria |
|---------|-----------|------------------|---------------|
| ET-001 | Temperature | Operation 0-40°C | All functions normal |
| ET-002 | Humidity | 85% RH, non-condensing | No corrosion/failure |
| ET-003 | Vibration | Portable handling | No relay chatter |
| ET-004 | ESD | IEC 61000-4-2 | ±4kV contact, ±8kV air |

---

## Appendix A: Reference Documents

1. uSDX/truSDX Schematic - DL2MAN/PE1NNZ
2. Si5351A/B/C Datasheet - Skyworks Solutions
3. BQ25713/B Datasheet - Texas Instruments
4. BQ76920 Datasheet - Texas Instruments
5. STM32G4 Reference Manual - STMicroelectronics
6. TCPP03-M20 Datasheet - STMicroelectronics
7. uP9636 Datasheet - uPI Semiconductor
8. USB Type-C Specification Rev 2.0
9. USB Power Delivery Specification Rev 3.1

---

## Appendix B: Schematic Checklist

- [ ] STM32G4 with UCPD connections to USB-C #1
- [ ] TCPP03-M20 protection circuit
- [ ] BQ25713 charger with external MOSFETs
- [ ] BQ76920 battery AFE with cell connections
- [ ] FT4232H with USB-C #2
- [ ] Si5351A with crystal and I2C
- [ ] uP9636 H-bridge with bootstrap caps
- [ ] 5-band LPF with latching relays
- [ ] SWR bridge with ADC connections
- [ ] Audio input/output circuits
- [ ] Power distribution (5V, 3.3V rails)
- [ ] Decoupling capacitors on all ICs
- [ ] ESD protection on all external interfaces

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | Dec 2024 | - | Initial release |

---

*End of Document*
