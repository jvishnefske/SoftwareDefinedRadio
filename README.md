# SDR Transceiver

A compact, USB-C powered software-defined radio transceiver for amateur HF bands. Built around the STM32G474 microcontroller with integrated DSP, this radio covers 80m through 15m bands with CW, SSB, and AM modes. Features USB audio for digital modes (FT8/WSJT-X), CAT control via TS-480 protocol, and a 4S Li-Ion battery with USB-PD charging up to 60W.

## Project Status

**Schematic: Not Implemented**

The KiCad schematic is in early development. Hierarchical sheets have been created and components placed, but wiring and ERC validation are incomplete. See `design.md` for current status and `CLAUDE.md` for the detailed workflow.

**Firmware: Feature Complete**

The Rust firmware passes 594 unit tests covering DSP, radio control, and protocol handling. Embedded build compiles to 14KB flash.

## Features

- 80m, 40m, 30m, 20m, 17m, 15m bands
- CW, LSB, USB, AM modes
- 0.5W - 5W adjustable output
- Si5351A clock synthesis
- USB audio (48kHz) for digital modes
- CAT control (TS-480 compatible)
- USB-PD charging (20V/3A)
- 4S Li-Ion battery with BMS
- SWR protection with auto power reduction

## Directory Structure

```
sdr_kicad_project/
├── sdr_kicad/          # KiCad project and schematics
├── custom_lib/         # Custom symbols and footprints
├── firmware/           # Rust embedded firmware
├── kicad_lib/          # Python KiCad helper library
└── design.md           # Requirements and status tracking
```

## License

TBD
