# SDR Transceiver KiCad Project

## Overview

This is a KiCad 8.0 schematic for a Software Defined Radio (SDR) transceiver based on the uSDX/truSDX H-bridge Class-E power amplifier architecture.

## Key Features

- **STM32G474RET6** MCU with integrated USB Power Delivery (UCPD)
- **Dual USB-C connectors**: Power/Data + Debug/JTAG
- **3S Li-Ion battery** with BQ25713 buck-boost charger and BQ76920 AFE
- **Si5351A** programmable clock synthesizer for quadrature generation
- **FT4232H** quad USB-UART/JTAG debug interface
- **5-band LPF** with latching relay switching
- **H-Bridge PA** using uP9636 or discrete MOSFETs

## File Structure

```
sdr_kicad/
├── sdr_transceiver.kicad_pro      # KiCad project file
├── sdr_transceiver.kicad_sch      # Main schematic (top-level)
├── power_management.kicad_sch     # USB-C PD, charger, regulators
├── mcu_stm32g4.kicad_sch          # STM32G474 microcontroller
├── clock_si5351.kicad_sch         # Si5351A clock synthesizer
├── rf_section.kicad_sch           # H-bridge PA, LPF, T/R switch
├── usb_connectors.kicad_sch       # Dual USB-C connectors
├── debug_ft4232.kicad_sch         # FT4232H debug interface
├── BOM_JLCPCB.csv                 # Bill of Materials for JLCPCB
└── README.md                       # This file
```

## JLCPCB Part Selection

Parts have been selected prioritizing JLCPCB Basic parts where possible:

### Basic Parts (No Extended Fee)
- Resistors: 0402 size (C25744, C25905, C25879, etc.)
- Capacitors: 0402/0603/0805 MLCC (C1525, C19702, C45783)
- Transistors: 2N7002 (C8545), SI2302 (C10487)
- Diodes: SS34 Schottky (C8678), 1N4148 (C81598)
- LDO: AMS1117-3.3 (C6186)
- Op-Amp: LM358 (C7950)
- ESD: USBLC6-2SC6 (C7519)
- Crystals: 8MHz HC-49S (C32160)
- Ferrite Beads: (C1015)

### Extended Parts (Required for Function)
- STM32G474RET6 (C481410)
- Si5351A-B-GT (C506891)
- BQ25713RSNR (C134263)
- BQ76920PW (C82092)
- TCPP03-M20 (C2682766)
- FT4232HL (C2688064)
- USB-C Connectors (C2765186)
- Latching Relays (C132490)

## Power Architecture

```
USB-C VBUS (5-20V PD)
    │
    ├─► TCPP03-M20 (Protection)
    │       │
    │       └─► STM32G4 UCPD (Negotiation)
    │
    └─► BQ25713 (Buck-Boost Charger)
            │
            ├─► VSYS (9-14V) ──► H-Bridge PA
            │                    │
            │                    └─► 5V Buck ──► FT4232H, Si5351 VDDO
            │
            └─► Battery ──► BQ76920 (AFE/Protection)
                    │
                    └─► 3.3V LDO ──► STM32G4, Si5351 Core
```

## RF Section

- **H-Bridge PA**: uP9636 or discrete Si2302 + IR2104
- **Output Power**: 0.5-5W adjustable
- **LPF Switching**: 5 latching relays (zero standby current)
- **Bands**: 80m, 40m, 30/20m, 17/15m, 12/10m

## I2C Bus Devices

| Address | Device | Function |
|---------|--------|----------|
| 0x60 | Si5351A | Clock Synthesizer |
| 0x6B | BQ25713 | Battery Charger |
| 0x08 | BQ76920 | Battery AFE |
| 0x35 | TCPP03-M20 | USB-C Protection |
| 0x3C | SSD1306 | OLED Display (optional) |

## Debug Interface

FT4232H provides:
- Channel A/B: JTAG/SWD to STM32G4
- Channel C: Debug UART (115200 baud)
- Channel D: CAT Control UART

## Usage Notes

1. **Opening the Project**: Use KiCad 8.0 or later
2. **Symbol Libraries**: Uses KiCad standard libraries
3. **Custom Symbols**: Some ICs may need custom symbols created
4. **Footprints**: Standard KiCad footprints used where possible

## Estimated BOM Cost

| Category | Cost |
|----------|------|
| ICs and Active | ~$35 |
| Passives | ~$5 |
| Connectors | ~$3 |
| Relays | ~$4 |
| Crystals | ~$1 |
| **Total** | **~$48** |

*Note: Excludes PCB, battery, and mechanical parts*

## JLCPCB Assembly Notes

1. Export BOM and CPL files from KiCad
2. Use BOM_JLCPCB.csv as reference for part numbers
3. Basic parts have no extended fee ($0)
4. Extended parts incur $3 fee per unique part
5. Consider hand-soldering extended parts to save cost

## License

This design is provided as-is for educational and amateur radio use.

## References

- uSDX Project: https://github.com/threeme3/usdx
- truSDX: https://dl2man.de/
- Si5351 Library: https://github.com/etherkit/Si5351Arduino
- STM32 USB-PD: ST X-CUBE-TCPP software package
