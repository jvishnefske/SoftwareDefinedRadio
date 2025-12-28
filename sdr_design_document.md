# Software Defined Radio Design Document

## Project: STM32-Based SDR Transceiver with USB-C Power Delivery

**Version:** 2.0
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

- **STM32G4 microcontroller** for DSP and system control (UCPD peripheral unused)
- **TPS25750D standalone USB-PD controller** for autonomous power negotiation
- **Dual USB-C connectors**: one for power/data, one for debug/JTAG
- **4S lithium battery** (12.8-16.8V) with BQ25798 buck-boost charging and BQ76920 monitoring
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
│  ┌──────────────┐     ┌──────────────────────────────────────────────────┐  │
│  │  USB-C #1    │     │              POWER MANAGEMENT                    │  │
│  │  (Power+Data)│────►│  TPS25750D (Standalone USB-PD) ──► BQ25798       │  │
│  │  Up to 20V   │     │  (I2CM autonomous charger control)     │         │  │
│  └──────────────┘     │                                        ▼         │  │
│                       │                              4S Li-Ion Pack      │  │
│                       │                              (12.8-16.8V)        │  │
│                       │                                    │             │  │
│                       │                              BQ76920 AFE         │  │
│                       │                              (Battery Monitor)   │  │
│                       └──────────────────────────────────────────────────┘  │
│                                         │                                   │
│                                         ▼                                   │
│  ┌──────────────┐     ┌──────────────────────────────────────────────────┐  │
│  │  USB-C #2    │     │              DIGITAL SUBSYSTEM                   │  │
│  │  (Debug)     │────►│  FT4232H ◄──► STM32G474 ◄──► Si5351A             │  │
│  │  JTAG+Serial │     │  (JTAG/UART)  (Main MCU)     (Clock Synth)       │  │
│  └──────────────┘     └──────────────────────────────────────────────────┘  │
│                                         │                                   │
│                                         ▼                                   │
│                       ┌──────────────────────────────────────────────────┐  │
│                       │              RF SUBSYSTEM                        │  │
│                       │                                                  │  │
│                       │  ┌─────────┐   ┌─────────┐   ┌─────────────────┐ │  │
│                       │  │Quadrature   │ H-Bridge│   │  LPF Bank       │ │  │
│                       │  │Sampling │◄─►│ PA      │──►│  (5 bands)      │ │  │
│                       │  │Detector │   │ uP9636  │   │  Relay/MOSFET   │ │  │
│                       │  └─────────┘   └─────────┘   └────────┬────────┘ │  │
│                       │       │                               │          │  │
│                       │       │         ┌─────────────────────┘          │  │
│                       │       ▼         ▼                                │  │
│                       │  ┌─────────────────────┐                         │  │
│                       │  │    SWR Bridge       │──────► ANT              │  │
│                       │  │  (Power/SWR Meter)  │                         │  │
│                       │  └─────────────────────┘                         │  │
│                       └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Key Features

| Feature | Specification |
|---------|---------------|
| Frequency Range | 3.5 - 30 MHz (80m - 10m) |
| Output Power | 0.5 - 5W (adjustable) |
| Modes | CW, LSB, USB, AM, FM, Digital |
| Power Input | USB-C PD (5-20V via TPS25750D), 4S Li-Ion |
| Battery | 4S 18650 (12.8-16.8V, ~3000mAh) |
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
| PR-001 | Shall negotiate USB PD up to 20V/3A (60W max) via TPS25750D | Must |
| PR-002 | Shall charge 4S Li-Ion pack with CC/CV profile via BQ25798 | Must |
| PR-003 | Shall provide battery protection (OVP, UVP, OCP, OTP) via BQ76920 | Must |
| PR-004 | Shall report battery state via I2C/SMBus | Should |
| PR-005 | Shall support dead battery boot from USB-C (TPS25750D feature) | Must |
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

**Rationale:** The STM32G4 series provides HRTIM for precise PWM generation and sufficient DSP capability for audio processing. Note: While the STM32G4 has integrated UCPD, this design uses a standalone TPS25750D for USB-PD to simplify firmware development and enable autonomous charger configuration.

### 4.2 USB-C Port Architecture

#### Port 1: Power + MCU USB (Primary)

```
USB-C Connector #1
        │
        ├── CC1/CC2 ──► TPS25750D (direct connection)
        │                   │
        │                   ├── Integrated CC protection
        │                   ├── Integrated 28V/7A power switch
        │                   ├── Dead Battery Support
        │                   └── I2CM port → BQ25798 (autonomous charger config)
        │
        ├── VBUS ──► TPS25750D ──► PP_HV output ──► BQ25798 (Charger Input)
        │                                              │
        │                                              └──► VSYS Power Rail (12-18V)
        │
        ├── D+/D- ──► STM32G4 USB-FS
        │              (CDC-ACM for CAT control)
        │
        └── SBU1/SBU2 ── (Reserved for future use)

Note: No TCPP03 or STM32 UCPD needed - TPS25750D handles all USB-PD functions.
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
                                    ┌─────────────────────────────────────────┐
USB-C VBUS ─────────────────────────┤           TPS25750D                     │
(5-20V from PD)                     │    Standalone USB-PD Controller         │
        │                           │                                         │
        │                           │  • Integrated 28V/7A power switch       │
        │                           │  • Integrated CC protection             │
        │   ┌───────────────────────┤  • Dead battery boot support            │
        │   │                       │  • I2CM port: autonomous BQ25798 ctrl   │
        │   │                       │  • I2CS port: STM32 monitoring          │
        │   │                       │                                         │
        │   │                       │ PP_HV ──────────────────────┐           │
        │   │                       └─────────────────────────────┼──────────┘
        │   │                                                     │
        │   │                                                     ▼
        │   │                       ┌─────────────────────────────────────────┐
        │   │                       │           BQ25798                       │
        │   │                       │    1-4S Buck-Boost Charger              │
        │   │                       │                                         │
        │   │                       │ VAC1 ◄── PP_HV from TPS25750D           │
        │   │                       │                                         │
        │   │                       │ SYS ─────────────────────► VSYS         │
        │   │                       │                            (12-18V)     │
        │   │                       │ BAT ◄────────────────┐                  │
        │   │                       │                      │                  │
        │   │                       │ I2C ◄─── TPS25750D I2CM (autonomous)    │
        │   │                       └──────────────────────┼─────────────────┘
        │   │                                              │
        │   │                                              ▼
        │   │                           ┌─────────────────────────┐
        │   │                           │    4S Li-Ion Pack       │
        │   │                           │   (12.8V - 16.8V)       │
        │   │                           │                         │
        │   │                           │  Cell 1 ─┬─ 3.2-4.2V    │
        │   │                           │  Cell 2 ─┼─ 3.2-4.2V    │
        │   │                           │  Cell 3 ─┼─ 3.2-4.2V    │
        │   │                           │  Cell 4 ─┴─ 3.2-4.2V    │
        │   │                           └─────────┬───────────────┘
        │   │                                     │
        │   │                                     ▼
        │   │                           ┌─────────────────────────┐
        │   │                           │     BQ76920             │
        │   │                           │  Battery AFE (4S mode)  │
        │   │                           │                         │
        │   │                           │ • Cell balancing        │
        │   │                           │ • Voltage monitor       │
        │   │                           │ • Coulomb counter       │
        │   │                           │ • Temp sensing          │
        │   │                           │ • I2C telemetry         │
        │   │                           └─────────────────────────┘
        │   │
        ▼   ▼
┌───────────────────────────────────────────────────────────────────┐
│                        POWER DISTRIBUTION                         │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│   VSYS (12-18V) ─┬──► H-Bridge PA (uP9636)                        │
│                  │                                                │
│                  └──► 5V Buck ──┬──► FT4232H                      │
│                      (XL1509)   │                                 │
│                                 ├──► Si5351A (VDDO)               │
│                                 │                                 │
│                                 ├──► USB VBUS (OTG mode)          │
│                                 │                                 │
│                                 └──► 3.3V LDO ──┬──► STM32G4      │
│                                     (AMS1117)   │                 │
│                                                 ├──► Si5351A VDD  │
│                                                 │                 │
│                                                 └──► Misc analog  │
│                                                                   │
│   NOTE: AMS1117-3.3 input from 5V rail (NOT VSYS) - max 15V limit │
└───────────────────────────────────────────────────────────────────┘
```

### 5.2 USB Power Delivery Negotiation

The TPS25750D standalone PD controller handles all USB-PD negotiation autonomously:

**TPS25750D Operation:**
- Fully autonomous USB-PD negotiation (no MCU involvement required)
- Configured via internal OTP or external I2C EEPROM
- I2CM port directly controls BQ25798 charger via I2C
- I2CS port provides STM32 with status monitoring

**Supported Power Profiles (Sink):**

| PDO | Voltage | Current | Power |
|-----|---------|---------|-------|
| 1 | 5V | 3A | 15W |
| 2 | 9V | 3A | 27W |
| 3 | 12V | 3A | 36W |
| 4 | 15V | 3A | 45W |
| 5 | 20V | 3A | 60W |

**Charging Strategy:**

1. At cable connection, TPS25750D negotiates highest available voltage
2. TPS25750D configures BQ25798 via I2CM for 4S charging (16.8V, CC/CV)
3. Charge current automatically limited based on negotiated power
4. PP_HV output provides switched VBUS to BQ25798 VAC1 input
5. System runs from VSYS (12-18V) via BQ25798 NVDC topology

**Dead Battery Boot:**
- TPS25750D enables system boot even with fully depleted battery
- Provides minimum power to 3.3V rail before battery reaches viable charge

### 5.3 Battery Management

**BQ76920 Configuration (4S Mode):**

| Parameter | Setting |
|-----------|---------|
| Cell Count | 4S |
| Overvoltage | 4.25V/cell (17.0V pack) |
| Undervoltage | 2.8V/cell (11.2V pack) |
| Overcurrent (Discharge) | 10A |
| Overcurrent (Charge) | 3A |
| Short Circuit | 50A, 200µs |
| Temperature Range | 0-45°C (charge), -20-60°C (discharge) |

**BQ25798 Charger Configuration:**

| Parameter | Setting |
|-----------|---------|
| Charge Voltage | 16.8V (4S x 4.2V) |
| Charge Current | Up to 3A (negotiated) |
| Input Voltage Range | 5-20V (from TPS25750D PP_HV) |
| VSYS Output | 12-18V |
| Topology | Buck-Boost NVDC |

**Telemetry (I2C readable):**

- Individual cell voltages (BQ76920)
- Pack voltage and current (BQ76920)
- State of charge (Coulomb counting)
- Temperature (NTC)
- Fault status flags
- Charger status (BQ25798 via STM32 I2C1)

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

**Dual-Master Architecture:**

```
                    TPS25750D
                        │
            +-----------+-----------+
            |                       |
        I2CS Port               I2CM Port
       (Host/Monitor)         (Charger Control)
            |                       |
            v                       v
       STM32 I2C1              BQ25798 only
       (shared bus)             (0x6B)
            |
       +----+----+----+----+
       |    |    |    |    |
    Si5351 BQ76920 TPS25750 SSD1306
    (0x60) (0x08)  (0x21)  (0x3C)
```

**STM32 I2C1 Bus @ 400 kHz:**

| Device | Address | Function |
|--------|---------|----------|
| Si5351A | 0x60 | Clock synthesizer |
| BQ76920 | 0x08 | Battery AFE |
| TPS25750D | 0x21 | PD controller (I2CS - monitoring) |
| BQ25798 | 0x6B | Battery charger (read-only monitoring) |
| SSD1306 | 0x3C | OLED display (optional) |

**TPS25750D I2CM (Autonomous):**

| Device | Address | Function |
|--------|---------|----------|
| BQ25798 | 0x6B | Charger configuration & control |

Note: TPS25750D I2CM operates independently to configure BQ25798 based on
PD negotiation results. STM32 can monitor charger status but TPS25750D
has priority for charger control.

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
| TPS25750D_INT | PB4 | Input | PD controller interrupt (optional) |
| GPIO_SPARE | PB5 | Reserved | Available for expansion |
| USB DP | PA12 | AF | USB data + |
| USB DM | PA11 | AF | USB data - |

Note: PB4/PB5 were previously allocated to UCPD CC lines. With TPS25750D
handling USB-PD autonomously, these pins are now available. PB4 can be
used to receive interrupts from TPS25750D for status changes.

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
│  2. TPS25750D enables power path (autonomous)                           │
│     │                                                                   │
│     ├── If USB: TPS25750D negotiates PD, enables PP_HV                  │
│     │           Configures BQ25798 via I2CM (no MCU needed)             │
│     │                                                                   │
│     └── If battery only: BQ25798 provides VSYS from battery             │
│     │                                                                   │
│     ▼                                                                   │
│  3. XL1509 provides 5V → AMS1117 provides 3.3V → STM32G4 boots          │
│     │                                                                   │
│     ▼                                                                   │
│  4. STM32 initializes I2C1 peripheral                                   │
│     │                                                                   │
│     ├── Read TPS25750D status (PD contract info)                        │
│     │                                                                   │
│     ├── Read BQ25798 status (charging state)                            │
│     │                                                                   │
│     └── Read BQ76920 for SOC                                            │
│         │                                                               │
│         ▼                                                               │
│         If SOC < 10%: Display low battery warning                       │
│     │                                                                   │
│     ▼                                                                   │
│  5. Initialize Si5351A clock synthesizer                                │
│     │                                                                   │
│     ▼                                                                   │
│  6. Load last-used frequency and mode from flash                        │
│     │                                                                   │
│     ▼                                                                   │
│  7. Select appropriate LPF (pulse latching relay)                       │
│     │                                                                   │
│     ▼                                                                   │
│  8. Enable receiver, begin audio processing                             │
│     │                                                                   │
│     ▼                                                                   │
│  9. OPERATIONAL - Ready for RX/TX                                       │
│                                                                         │
│  Note: PD negotiation and charger config happen BEFORE MCU boots.       │
│        MCU only monitors - no USB-PD firmware stack required.           │
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
│  T/R Switch (in RX position)                                            │
│     │                                                                   │
│     ▼                                                                   │
│  Band-Pass Filter (selected for current band)                           │
│     │                                                                   │
│     ▼                                                                   │
│  Quadrature Sampling Detector                                           │
│     │                                                                   │
│     ├──► I Channel ──► Anti-alias LPF ──► STM32 ADC1                    │
│     │                                                                   │
│     └──► Q Channel ──► Anti-alias LPF ──► STM32 ADC2                    │
│                                           │                             │
│                                           ▼                             │
│                              ┌─────────────────────────┐                │
│                              │    DSP Processing       │                │
│                              │                         │                │
│                              │  • Hilbert transform    │                │
│                              │  • SSB demodulation     │                │
│                              │  • CW filtering         │                │
│                              │  • AGC                  │                │
│                              │  • Noise reduction      │                │
│                              └───────────┬─────────────┘                │
│                                          │                              │
│                                          ▼                              │
│                              DAC ──► Audio Amplifier ──► Speaker        │
│                                                                         │
│  Concurrent Tasks:                                                      │
│  • S-meter calculation from AGC level                                   │
│  • Spectrum display update (optional)                                   │
│  • Frequency display update                                             │
│  • Battery SOC monitoring                                               │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.3 Transmit Operation

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         TRANSMIT OPERATION                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  PTT Asserted (hardware interrupt)                                      │
│     │                                                                   │
│     ▼                                                                   │
│  TX Sequence Start                                                      │
│     │                                                                   │
│     ├─► 1. Disable receiver audio output (mute)                         │
│     │                                                                   │
│     ├─► 2. Switch T/R relay to TX (if not using QSK)                    │
│     │                                                                   │
│     ├─► 3. Configure Si5351 for TX frequency                            │
│     │      (may include RIT/XIT offset)                                 │
│     │                                                                   │
│     ├─► 4. Enable PA bias (soft start)                                  │
│     │                                                                   │
│     ├─► 5. Begin audio sampling and modulation                          │
│     │                                                                   │
│     └─► 6. Monitor SWR continuously                                     │
│            │                                                            │
│            ├── If SWR > 3:1: Reduce power                               │
│            └── If SWR > 5:1: Disable TX, alert user                     │
│                                                                         │
│  During TX:                                                             │
│     │                                                                   │
│     ▼                                                                   │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    MODULATION MODES                             │    │
│  ├─────────────────────────────────────────────────────────────────┤    │
│  │                                                                 │    │
│  │  CW Mode:                                                       │    │
│  │    Key input ──► PWM gate ──► H-Bridge ──► LPF ──► ANT          │    │
│  │    (shaped envelope for click-free keying)                      │    │
│  │                                                                 │    │
│  │  SSB Mode:                                                      │    │
│  │    Mic ──► ADC ──► DSP (Hilbert/Weaver) ──► PWM ──► H-Bridge    │    │
│  │    (amplitude modulation of Class E PA)                         │    │
│  │                                                                 │    │
│  │  Digital Mode:                                                  │    │
│  │    USB Audio ──► ADC ──► PWM ──► H-Bridge ──► LPF ──► ANT       │    │
│  │    (constant carrier, audio modulation)                         │    │
│  │                                                                 │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                         │
│  PTT Released                                                           │
│     │                                                                   │
│     ▼                                                                   │
│  TX Sequence End                                                        │
│     │                                                                   │
│     ├─► 1. Ramp down PA (soft stop)                                     │
│     │                                                                   │
│     ├─► 2. Disable PA bias                                              │
│     │                                                                   │
│     ├─► 3. Switch T/R relay to RX                                       │
│     │                                                                   │
│     ├─► 4. Restore RX frequency (remove RIT/XIT)                        │
│     │                                                                   │
│     └─► 5. Re-enable receiver audio                                     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.4 Band Change Operation

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         BAND CHANGE SEQUENCE                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  User requests band change (encoder/button/CAT)                         │
│     │                                                                   │
│     ▼                                                                   │
│  1. Verify not in TX mode (abort if PTT active)                         │
│     │                                                                   │
│     ▼                                                                   │
│  2. Save current frequency to band memory                               │
│     │                                                                   │
│     ▼                                                                   │
│  3. Mute audio output                                                   │
│     │                                                                   │
│     ▼                                                                   │
│  4. Determine new LPF required                                          │
│     │                                                                   │
│     ├── If same LPF: Skip to step 7                                     │
│     │                                                                   │
│     └── If different LPF:                                               │
│         │                                                               │
│         ▼                                                               │
│  5. Pulse RESET on current relay (opens current LPF)                    │
│     │                                                                   │
│     ▼                                                                   │
│  6. Pulse SET on new relay (closes new LPF)                             │
│     │                                                                   │
│     ▼                                                                   │
│  7. Update Si5351 to new frequency                                      │
│     │                                                                   │
│     ▼                                                                   │
│  8. Load saved frequency for new band (or band edge)                    │
│     │                                                                   │
│     ▼                                                                   │
│  9. Update display                                                      │
│     │                                                                   │
│     ▼                                                                   │
│  10. Un-mute audio                                                      │
│     │                                                                   │
│     ▼                                                                   │
│  Complete - Now receiving on new band                                   │
│                                                                         │
│  Timing: ~20ms total (dominated by relay settling)                      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.5 USB-C Power Events

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     USB-C POWER EVENT HANDLING                          │
│                   (TPS25750D Autonomous Operation)                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  EVENT: USB-C Cable Connected                                           │
│     │                                                                   │
│     ▼                                                                   │
│  TPS25750D detects CC connection (autonomous - no MCU needed)           │
│     │                                                                   │
│     ▼                                                                   │
│  TPS25750D negotiates PD contract (highest available PDO)               │
│     │                                                                   │
│     ▼                                                                   │
│  TPS25750D enables PP_HV output with negotiated voltage                 │
│     │                                                                   │
│     ▼                                                                   │
│  TPS25750D configures BQ25798 via I2CM:                                 │
│     • Set input voltage limit based on negotiated PDO                   │
│     • Set charge current based on power budget                          │
│     • Enable charging if battery < 16.8V                                │
│     │                                                                   │
│     ▼                                                                   │
│  TPS25750D asserts INT to notify STM32 (optional)                       │
│     │                                                                   │
│     ▼                                                                   │
│  STM32 reads status via I2CS, updates UI                                │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  EVENT: USB-C Cable Disconnected                                        │
│     │                                                                   │
│     ▼                                                                   │
│  TPS25750D detects CC disconnect                                        │
│     │                                                                   │
│     ▼                                                                   │
│  TPS25750D disables PP_HV output                                        │
│     │                                                                   │
│     ▼                                                                   │
│  BQ25798 automatically switches to battery power (NVDC)                 │
│     │                                                                   │
│     ▼                                                                   │
│  STM32 checks battery SOC via BQ76920                                   │
│     │                                                                   │
│     ├── If SOC > 20%: Continue normal operation                         │
│     │                                                                   │
│     └── If SOC < 20%: Display battery warning                           │
│         │                                                               │
│         └── If SOC < 5%: Initiate graceful shutdown                     │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  EVENT: Hard Reset / Power Source Change                                │
│     │                                                                   │
│     ▼                                                                   │
│  TPS25750D handles protocol-level reset autonomously                    │
│     │                                                                   │
│     ▼                                                                   │
│  BQ25798 maintains VSYS from battery (seamless NVDC)                    │
│     │                                                                   │
│     ▼                                                                   │
│  TPS25750D re-negotiates when source ready                              │
│                                                                         │
│  Note: STM32 firmware never involved in PD negotiation.                 │
│        All power events handled by TPS25750D hardware.                  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.6 Low Power / Sleep Mode

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         LOW POWER OPERATION                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Trigger: No user activity for configurable timeout (default 5 min)     │
│                                                                         │
│  SLEEP ENTRY:                                                           │
│     │                                                                   │
│     ├─► Disable Si5351 clock outputs                                    │
│     │                                                                   │
│     ├─► Power down audio amplifier                                      │
│     │                                                                   │
│     ├─► Disable OLED (or reduce brightness)                             │
│     │                                                                   │
│     ├─► STM32 enters STOP2 mode                                         │
│     │                                                                   │
│     └─► TPS25750D + BQ25798 continue charging autonomously              │
│                                                                         │
│  Wake Sources:                                                          │
│     • PTT input (GPIO EXTI)                                             │
│     • Encoder rotation                                                  │
│     • TPS25750D INT (USB-C power event)                                 │
│     • RTC alarm (periodic battery check)                                │
│                                                                         │
│  SLEEP CURRENT BUDGET:                                                  │
│     │                                                                   │
│     ├── STM32 STOP2: ~2 µA                                              │
│     ├── TPS25750D standby: ~15 µA                                       │
│     ├── BQ25798 standby: ~10 µA                                         │
│     ├── BQ76920 ship mode: ~2 µA                                        │
│     ├── Si5351 disabled: ~1 µA                                          │
│     └── Relay leakage: ~0 µA (latching, no holding current)             │
│         ─────────────────────                                           │
│         Total: ~30 µA                                                   │
│                                                                         │
│  Battery Life (sleep): 3000mAh / 0.030mA = ~11 years (theoretical)      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 9. Bill of Materials

### 9.1 Major Components

| Ref | Part Number | LCSC | Description | Qty | Est. Cost |
|-----|-------------|------|-------------|-----|-----------|
| U1 | STM32G474RET6 | C481410 | MCU, ARM Cortex-M4, 512K Flash | 1 | $8.50 |
| U2 | Si5351A-B-GT | C506891 | Clock synthesizer, I2C, 8 outputs | 1 | $2.50 |
| U3 | TPS25750DRJKR | C2868209 | Standalone USB-PD Controller | 1 | $1.70 |
| U4 | BQ25798RQMR | C2876593 | 1-4S Buck-Boost Charger, I2C | 1 | $1.47 |
| U5 | BQ76920PW | C82092 | Battery AFE, 3-5S, I2C (4S config) | 1 | $2.80 |
| U6 | FT4232HL | C2688064 | Quad USB-UART/JTAG | 1 | $5.50 |
| U7 | uP9636 | - | H-bridge driver w/ MOSFETs | 1 | $3.00 |
| U8 | XL1509-5.0 | C61063 | 5V 2A Buck Regulator (40V input) | 1 | $0.20 |
| U9 | AMS1117-3.3 | C6186 | 1A LDO, 3.3V (5V input) | 1 | $0.05 |
| J1,J2 | USB-C Receptacle | C2765186 | 16-pin, mid-mount | 2 | $1.00 |
| K1-K5 | EC2-3SNU | C132490 | Latching relay, DPDT, 3V | 5 | $4.00 |
| Y1 | 8MHz Crystal | C32160 | 20pF, ±20ppm, for STM32 | 1 | $0.05 |
| Y2 | 25MHz Crystal | C255909 | 18pF, ±10ppm, for Si5351 | 1 | $0.10 |

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
| ICs and Semiconductors | $32.00 |
| Passives | $18.00 |
| Connectors | $5.00 |
| PCB (4-layer, 100x80mm) | $15.00 |
| Enclosure | $10.00 |
| Battery (4S 18650) | $20.00 |
| **Total BOM** | **~$100** |

Note: TPS25750D ($1.70) + BQ25798 ($1.47) = $3.17 vs old TCPP03 ($1.50) + BQ25713 ($4.20) = $5.70.
Net savings of ~$2.50 offset by 4S battery cost increase.

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

### 10.2 Integrated UCPD vs Standalone PD Controller

| Factor | STM32 UCPD + TCPP03 | TPS25750D Standalone | Winner |
|--------|---------------------|----------------------|--------|
| BOM Cost | ~$1.50 (TCPP03) | ~$1.70 | Tie |
| Firmware Complexity | High (X-CUBE-TCPP, RTOS) | Zero (autonomous) | TPS25750D |
| Charger Integration | Manual I2C config | Autonomous I2CM bus | TPS25750D |
| Dead Battery Boot | Complex | Built-in | TPS25750D |
| Rust/Embedded | FFI to ST C stack | Pure I2C monitoring | TPS25750D |
| USB-IF Certification | ST-provided stack | TI-provided config | Tie |
| Power Path Control | Firmware-dependent | Hardware-controlled | TPS25750D |

**Decision: TPS25750D Standalone Controller**

For this Rust-based embedded project, the TPS25750D provides significant advantages:

1. **No USB-PD firmware required** - TPS25750D handles all PD negotiation autonomously
2. **Native I2C master** - Configures BQ25798 charger without MCU involvement
3. **Simpler boot sequence** - Power available before MCU even starts
4. **Dead battery boot** - Hardware-guaranteed, not firmware-dependent
5. **Eliminates FFI complexity** - No X-CUBE-TCPP C library bindings needed
6. **Reduced certification scope** - PD stack is TI's responsibility

The marginal cost increase ($0.20) is justified by the dramatic reduction in firmware complexity and improved system reliability.

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
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐  │
│  │   Radio   │ │   Power   │ │    UI     │ │    CAT    │ │  Config   │  │
│  │  Control  │ │  Monitor  │ │  Manager  │ │  Handler  │ │  Storage  │  │
│  └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘  │
│        │             │             │             │             │        │
├────────┴─────────────┴─────────────┴─────────────┴─────────────┴────────┤
│                         MIDDLEWARE LAYER                                │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐                │
│  │   I2C     │ │   Audio   │ │   DSP     │ │  Embassy  │                │
│  │  Drivers  │ │  Codec    │ │  Library  │ │async RTOS │                │
│  │(TPS/BQ/SI)│ │           │ │  (CMSIS)  │ │           │                │
│  └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘                │
│        │             │             │             │                      │
├────────┴─────────────┴─────────────┴─────────────┴──────────────────────┤
│                           HAL LAYER (Rust)                              │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐                │
│  │ ADC │ │ DAC │ │HRTIM│ │ I2C │ │ SPI │ │ USB │ │GPIO │                │
│  └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘                │
│     │       │       │       │       │       │       │                   │
├─────┴───────┴───────┴───────┴───────┴───────┴───────┴──────────────────┤
│                         HARDWARE                                        │
│  Note: USB-PD handled entirely by TPS25750D - no firmware required      │
└─────────────────────────────────────────────────────────────────────────┘
```

**Key Simplification:** With TPS25750D handling USB-PD autonomously, the firmware
stack eliminates the X-CUBE-TCPP dependency and any C FFI bindings. Power
monitoring is reduced to simple I2C register reads.

### 11.2 RTOS Task Structure

| Task | Priority | Stack | Period | Function |
|------|----------|-------|--------|----------|
| DSP_Audio | Highest | 2KB | 48kHz interrupt | Audio sample processing |
| Radio_Control | High | 1KB | Event-driven | TX/RX state machine |
| Power_Monitor | Medium | 512B | 1000ms | Battery/charger I2C polling |
| UI_Update | Low | 1KB | 50ms | Display refresh |
| CAT_Handler | Low | 1KB | Event-driven | Serial command processing |
| Idle | Lowest | 256B | N/A | Sleep mode entry |

Note: USB_PD task eliminated - TPS25750D handles PD autonomously.
Power_Monitor only polls I2C status registers, no complex state machine.

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
| UT-003 | BQ25798 | I2C communication | Read/write all registers |
| UT-004 | BQ25798 | Charge profile | CC/CV within 1% (4S/16.8V) |
| UT-005 | TPS25750D | PD negotiation | Contract at 20V/3A |
| UT-006 | TPS25750D | I2CM to BQ25798 | Autonomous charger config |
| UT-007 | LPF Relay | Switching | <10ms, reliable |
| UT-008 | H-Bridge | PWM output | 1-30 MHz, clean edges |

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
3. TPS25750 Datasheet - Texas Instruments (SLVSFP8)
4. BQ25798 Datasheet - Texas Instruments (SLUSE18)
5. BQ76920 Datasheet - Texas Instruments
6. STM32G4 Reference Manual - STMicroelectronics
7. uP9636 Datasheet - uPI Semiconductor
8. USB Type-C Specification Rev 2.0
9. USB Power Delivery Specification Rev 3.1
10. TPS25750 Application Report - TI (SLVAF56)

---

## Appendix B: Schematic Checklist

- [x] STM32G4 MCU with I2C1 connections
- [x] TPS25750D standalone USB-PD controller (LCSC: C2868209)
- [x] BQ25798 1-4S buck-boost charger (LCSC: C2876593)
- [x] BQ76920 battery AFE with 4S cell connections
- [x] FT4232H with USB-C #2
- [x] Si5351A with crystal and I2C
- [ ] uP9636 H-bridge with bootstrap caps
- [ ] 5-band LPF with latching relays
- [ ] SWR bridge with ADC connections
- [ ] Audio input/output circuits
- [x] Power distribution (5V from XL1509, 3.3V from AMS1117)
- [ ] Decoupling capacitors on all ICs
- [x] ESD protection on USB data lines (USBLC6-2SC6)
- [x] AMS1117-3.3 input from 5V rail (NOT VSYS)
- [x] TPS25750D I2CM bus to BQ25798
- [x] TPS25750D I2CS bus to STM32 I2C1
- [ ] Verify uP9636 VCC rating >= 18V for 4S operation

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | Dec 2024 | - | Initial release |
| 2.0 | Dec 2024 | - | Major architecture update: TPS25750D + BQ25798 replaces TCPP03 + BQ25713; 4S battery (12.8-16.8V); AMS1117 input from 5V rail; eliminated UCPD firmware dependency |

---

*End of Document*
