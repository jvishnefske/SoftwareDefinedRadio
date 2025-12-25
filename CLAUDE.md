# KiCad Schematic and DFM Workflow

## Project Structure
```
sdr_kicad_project/
├── sdr_kicad/                    # Main KiCad project
│   ├── sdr_transceiver.kicad_pro # Project file
│   ├── sdr_transceiver.kicad_sch # Main hierarchical schematic
│   └── *.kicad_sch               # Subsystem sheets
├── custom_lib/
│   ├── symbol/                   # Custom schematic symbols
│   ├── footprint/                # Custom PCB footprints
│   └── footprint/packages3d/     # 3D STEP models
├── kicad_lib/                    # Python helper library
│   ├── types/                    # Immutable dataclasses
│   ├── parser/                   # S-expression and file parsers
│   ├── writer/                   # File serializers
│   ├── builders/                 # Fluent API builders
│   └── validation/               # ERC rules and runner
└── output/                       # Generated manufacturing files
```

## Component Import with JLC2KiCadLib

Import components from JLCPCB/LCSC with symbols, footprints, and 3D models:

```bash
uvx --from jlc2kicadlib JLC2KiCadLib <LCSC_PART_NUMBER> \
  -dir custom_lib \
  -symbol_lib_dir custom_lib/symbol \
  -footprint_lib custom_lib/footprint \
  -model_dir custom_lib/footprint/packages3d \
  -models STEP
```

Example for multiple parts:
```bash
uvx --from jlc2kicadlib JLC2KiCadLib C2040 C15850 C25744 \
  -dir custom_lib \
  -symbol_lib_dir custom_lib/symbol \
  -footprint_lib custom_lib/footprint \
  -model_dir custom_lib/footprint/packages3d \
  -models STEP
```

After import, add library to KiCad:
1. Preferences → Manage Symbol Libraries → Add row with path to `.kicad_sym`
2. Preferences → Manage Footprint Libraries → Add row with path to footprint folder

## Schematic Conventions

### Hierarchical Sheets
Name pattern: `{function}_{primary_ic}.kicad_sch`
- `power_management.kicad_sch` - Power rails, charging, regulation
- `mcu_stm32g4.kicad_sch` - Microcontroller and peripherals
- `rf_section.kicad_sch` - RF amplifier and filters
- `clock_si5351.kicad_sch` - Clock generation
- `debug_ft4232.kicad_sch` - Debug interfaces

### Reference Designators
- U: ICs (MCU, regulators, drivers)
- Q: Transistors, MOSFETs
- R: Resistors
- C: Capacitors
- L: Inductors
- D: Diodes
- K: Relays
- J: Connectors
- Y: Crystals
- FB: Ferrite beads
- SW: Switches

### Power Rails
- `VBUS` - USB input (5-20V PD)
- `VSYS` - System bus (9-14V)
- `+5V` - 5V logic supply
- `+3V3` - 3.3V MCU/analog supply
- `VBAT+` - Battery positive
- `GND` - Ground reference

### Signal Naming
- Bus notation: `SIGNAL[0..N]` (e.g., `BAND_SEL[0..4]`)
- Differential pairs: `SIG_P`, `SIG_N`
- Active low: `~{SIGNAL}` or `SIGNAL_N`

## ERC Verification

Before any PCB work, ERC must pass with zero errors:

```bash
# Export ERC report from command line
kicad-cli sch erc sdr_kicad/sdr_transceiver.kicad_sch \
  -o output/erc_report.txt \
  --exit-code-violations
```

Common ERC fixes:
- Add `PWR_FLAG` on power inputs from connectors
- Use `no_connect` flags on unused pins
- Ensure hierarchical labels match between sheets
- Check power pin connections (output to output conflicts)

## DFM - Design for Manufacturing (JLCPCB)

### BOM Format
File: `sdr_kicad/BOM_JLCPCB.csv`

Required columns:
```csv
Comment,Designator,Footprint,LCSC Part
100nF,C1 C2 C3,0402,C1525
10K,R1 R2,0402,C25744
STM32G474RET6,U1,LQFP-64,C467136
```

### Generate BOM from KiCad
1. Tools → Generate Bill of Materials
2. Use plugin or export to CSV
3. Add LCSC part numbers manually or via lookup

### CPL (Component Placement List)
File: `output/assembly/positions.csv`

Required columns:
```csv
Designator,Mid X,Mid Y,Layer,Rotation
C1,10.5,20.3,top,0
U1,50.2,30.1,top,90
```

Export: File → Fabrication Outputs → Component Placement

### Gerber Export
Directory: `output/gerber/`

Export settings for JLCPCB:
- Format: Gerber X2 (or RS-274X)
- Coordinate format: 4.6 (units: mm)
- Include edge cuts layer

Required layers:
- `*-F_Cu.gbr` - Front copper
- `*-B_Cu.gbr` - Back copper
- `*-F_Silkscreen.gbr` - Front silkscreen
- `*-B_Silkscreen.gbr` - Back silkscreen
- `*-F_Mask.gbr` - Front solder mask
- `*-B_Mask.gbr` - Back solder mask
- `*-Edge_Cuts.gbr` - Board outline
- `*-F_Paste.gbr` - Front paste (for stencil)
- `*-B_Paste.gbr` - Back paste (for stencil)

### Drill Files
- Format: Excellon
- Combine PTH and NPTH into single file (JLCPCB preferred)
- Generate drill map for verification

```bash
# Command line Gerber/drill export
kicad-cli pcb export gerbers sdr_kicad/sdr_transceiver.kicad_pcb \
  -o output/gerber/ \
  --layers F.Cu,B.Cu,F.Silkscreen,B.Silkscreen,F.Mask,B.Mask,Edge.Cuts,F.Paste,B.Paste

kicad-cli pcb export drill sdr_kicad/sdr_transceiver.kicad_pcb \
  -o output/gerber/ \
  --format excellon \
  --generate-map
```

## Quality Gates

Before ordering:

1. **ERC Pass** - Zero errors (warnings acceptable with justification)
2. **DRC Pass** - Zero errors after PCB layout
3. **BOM Complete** - All SMD parts have LCSC numbers
4. **3D Verification** - Visual check of component fit
5. **Gerber Review** - Use online Gerber viewer or `gerbv`

## Output Directory Structure

```
output/
├── gerber/              # Fabrication Gerbers + drills
├── assembly/
│   ├── BOM_JLCPCB.csv   # Bill of materials
│   └── positions.csv    # Component placement
├── documentation/
│   ├── schematic.pdf    # Full schematic export
│   └── assembly.pdf     # Assembly drawings
└── 3d/
    └── board.step       # 3D model for enclosure design
```

## JLCPCB Upload Checklist

1. Zip all Gerber + drill files
2. Upload to jlcpcb.com → PCB Prototype
3. Enable SMT Assembly if needed
4. Upload BOM and CPL files
5. Review component placement in online viewer
6. Verify part availability and pricing

## Schematic Completion Status

### Current State (2024-12-25)

The schematic project contains:
- **7 hierarchical sheets** with proper sheet-pin connections in main sheet
- **Component symbols placed** on each sub-sheet
- **JLCPCB symbols downloaded** via JLC2KiCadLib for custom parts
- **Wire segments added** connecting hierarchical labels

### ERC Status (2025-12-25)

**Clock_Si5351 sheet**: 31 violations (14 errors, 17 warnings)
- Errors mainly from hierarchical labels (expected - sheet is standalone)
- Some wire endpoints need manual snapping to pins in KiCad GUI

**Full project**: Run `kicad-cli sch erc` on main sheet for full count

**Root Causes:**
1. **Wire endpoint misalignment** - Wires placed but not snapped to symbol pins
2. **Hierarchical labels in sub-sheet** - Expected when running ERC on sub-sheet alone
3. **Lib symbol cache** - Some symbols use cached copies that differ from library (warnings only)

**All custom JLCPCB symbols are present** - no missing symbols detected

### Symbol Plotting Fix Procedure

When symbols don't plot correctly (wires not connecting to pins):

1. **Find symbol position** from schematic file: `(at X Y rotation)`
2. **Find pin offset** from lib_symbols section: `(pin ... (at rel_x rel_y angle))`
3. **Calculate absolute position**:
   - `abs_x = symbol_x + rel_x`
   - `abs_y = symbol_y - rel_y` (Y is inverted in KiCad schematics)
4. **Draw wire** to exact coordinate, or use **net labels** to avoid long wire runs
5. **Verify with ERC**: `kicad-cli sch erc <file>`

### Required Manual Fixes (KiCad GUI)

Open `sdr_kicad/sdr_transceiver.kicad_sch` in KiCad and for each sub-sheet:

1. **Delete dangling wire segments** that don't connect to pins
2. **Draw new wires** from actual symbol pins to hierarchical labels
3. **Verify pin assignments** match the documented connections
4. **Run ERC** (Inspect → Electrical Rules Checker) after each sheet
5. **Add PWR_FLAG** symbols on power inputs from connectors

### Export Commands

```bash
# Export schematic PDF
/snap/bin/kicad.kicad-cli sch export pdf \
  -o output/sdr_schematic.pdf \
  sdr_kicad/sdr_transceiver.kicad_sch

# Run ERC
/snap/bin/kicad.kicad-cli sch erc \
  -o output/erc_report.txt \
  --exit-code-violations \
  sdr_kicad/sdr_transceiver.kicad_sch
```

---

## Per-Sheet Wiring Tasks

**Power_Management (power_management.kicad_sch)**
- [ ] Wire TPS25750DRJKR pins to hierarchical labels (VBUS, CC1, CC2, I2C, etc.)
- [ ] Wire BQ25798RQMR pins to power rails and I2C
- [ ] Wire XL1509 pins to VSYS input and +5V output
- [ ] Wire AMS1117-3.3 pins to +5V input and +3V3 output
- [ ] Add PWR_FLAG symbols on power inputs

**MCU_STM32G4 (mcu_stm32g4.kicad_sch)**
- [ ] Wire STM32G474RETx pins to hierarchical labels
- [ ] Connect crystal to OSC_IN/OSC_OUT
- [ ] Wire USB pins (PA11, PA12) to USB_DP, USB_DM labels
- [ ] Wire I2C pins (PB6, PB7) to I2C1_SCL, I2C1_SDA labels
- [ ] Wire HRTIM output to PWM_PA label
- [ ] Connect GPIO to BAND_SEL bus

**Clock_Si5351 (clock_si5351.kicad_sch)** - UPDATED 2024-12-25
- [x] Wire Si5351A SCL/SDA pins to I2C hierarchical labels (PC5_SCL, PC4_SDA)
- [x] Wire CLK0/CLK1/CLK2 outputs to hierarchical labels
- [x] Connect 27MHz crystal (Y1) to XA/XB pins
- [x] Wire VDD to +3V3
- [x] Wire VDDO through diodes D1/D2 for supply OR-ing
- [x] Add I2C pull-up resistors R4/R5 (1k)
- [x] Add decoupling capacitor C15 (100nF)
- [ ] **GUI FIX NEEDED**: Snap wire endpoints to pin locations in KiCad

### Clock_Si5351 Pin Coordinate Reference

Si5351A-B-GT at position (101.6, 101.6):

| Pin | Name | Absolute X | Absolute Y | Formula |
|-----|------|------------|------------|---------|
| 1   | VDD  | 91.44      | 96.52      | symbol_x + pin_x, symbol_y - pin_y |
| 2   | XA   | 91.44      | 99.06      | pin at (-10.16, 2.54) |
| 3   | XB   | 91.44      | 101.6      | pin at (-10.16, 0) |
| 4   | SCL  | 91.44      | 104.14     | pin at (-10.16, -2.54) |
| 5   | SDA  | 91.44      | 106.68     | pin at (-10.16, -5.08) |
| 6   | CLK2 | 111.76     | 106.68     | pin at (10.16, -5.08) |
| 7   | VDDO | 111.76     | 104.14     | pin at (10.16, -2.54) |
| 8   | GND  | 111.76     | 101.6      | pin at (10.16, 0) |
| 9   | CLK1 | 111.76     | 99.06      | pin at (10.16, 2.54) |
| 10  | CLK0 | 111.76     | 96.52      | pin at (10.16, 5.08) |

**Pin Position Formula**: `absolute = (symbol_x + relative_x, symbol_y - relative_y)`

**RF_Section (rf_section.kicad_sch)**
- [ ] Add PA MOSFET/driver symbols and wire to VSYS, PWM_IN
- [ ] Add LPF relay symbols and wire to BAND_SEL bus
- [ ] Add FST3253MX (QSD) and wire to CLK_I, CLK_Q inputs
- [ ] Wire RX_I, RX_Q outputs from QSD
- [ ] Add SWR bridge circuitry

**Debug_FT4232 (debug_ft4232.kicad_sch)**
- [ ] Wire FT4232HL pins to USB2_DP, USB2_DM
- [ ] Wire JTAG outputs (TCK, TMS, TDI, TDO) to hierarchical labels
- [ ] Add power decoupling capacitors

**USB_Connectors (usb_connectors.kicad_sch)**
- [ ] Add USB-C connector symbols (TYPE-C_16PIN_2MD)
- [ ] Wire VBUS, CC1, CC2, D+, D- to hierarchical labels
- [ ] Add USBLC6-2SC6 ESD protection on data lines

### ERC Fix Checklist
1. Ensure all hierarchical labels connect to wires on both ends
2. Add PWR_FLAG on power input pins from connectors
3. Use no_connect symbols on intentionally unused pins
4. Verify signal names match between main sheet pins and sub-sheet hierarchical labels

## Requirements Traceability

Functional requirements tracked in `design.md`. Mark requirements as validated when:
- Schematic implements the requirement
- ERC passes
- PCB layout complete
- DRC passes
- Test plan defined

---

## Python KiCad Helper Library (`kicad_lib`)

A Python library for programmatic creation, parsing, modification, and validation of KiCad files.

### Library Structure
```
kicad_lib/
├── __init__.py              # Public API exports
├── result.py                # Result[T, E] type for error handling
├── types/
│   ├── primitives.py        # Point, Position, UUID, Stroke, Fill, Effects
│   ├── enums.py             # PinElectricalType, PinShape, LabelShape, Severity
│   ├── symbol.py            # SymbolLibrary, SymbolDef, PinDef, graphics
│   └── schematic.py         # Schematic, Wire, Label, Sheet, SymbolInstance
├── parser/
│   ├── sexpr.py             # S-expression tokenizer and parser
│   ├── symbol_parser.py     # .kicad_sym parser
│   └── schematic_parser.py  # .kicad_sch parser
├── writer/
│   ├── sexpr_writer.py      # S-expression serializer
│   └── symbol_writer.py     # .kicad_sym writer
├── builders/
│   └── symbol_builder.py    # Fluent API for symbol creation
└── validation/
    └── erc.py               # ERC rules and runner
```

### Design Principles
- **Immutable types**: All dataclasses are frozen for thread safety
- **Result type**: Explicit error handling without exceptions
- **Builder pattern**: Fluent API for complex object construction
- **Round-trip support**: Parse → modify → write preserves structure

### Usage Examples

```python
# Load and inspect a schematic
from kicad_lib import load_schematic, run_erc

result = load_schematic("sdr_kicad/sdr_transceiver.kicad_sch")
if result.is_ok():
    sch = result.unwrap()
    print(f"Sheets: {len(sch.sheets)}, Wires: {len(sch.wires)}")

    # Run ERC validation
    erc = run_erc(sch)
    print(f"Errors: {erc.error_count}, Warnings: {erc.warning_count}")

# Load a symbol library
from kicad_lib import load_symbol_library

result = load_symbol_library("custom_lib/symbol/STM32G474RET6.kicad_sym")
lib = result.unwrap()
sym = lib.symbols[0]
print(f"Pins: {len(sym.all_pins())}")

# Create a symbol programmatically
from kicad_lib import SymbolBuilder, SymbolLibraryBuilder, write_symbol_library

symbol = (SymbolBuilder("LM7805")
    .with_reference("U")
    .with_footprint("Package_TO_SOT_THT:TO-220-3_Vertical")
    .with_property("LCSC", "C12345")
    .add_body_rectangle(15.24, 12.7)
    .add_pin_left("1", "VIN", 2.54, "power_in")
    .add_pin_left("2", "GND", -2.54, "power_in")
    .add_pin_right("3", "VOUT", 0, "power_out")
    .build())

lib = SymbolLibraryBuilder().add_symbol(symbol).build()
content = write_symbol_library(lib)
```

### Supported File Types
| Format | Parse | Write | Notes |
|--------|-------|-------|-------|
| .kicad_sch | ✓ | - | Schematic files (v20231120) |
| .kicad_sym | ✓ | ✓ | Symbol libraries (v20210201) |
| .kicad_pro | - | - | Project files (JSON) - planned |
| .kicad_mod | - | - | Footprints - planned |

### ERC Rules Implemented
| Rule ID | Severity | Description |
|---------|----------|-------------|
| wire_dangling | WARNING | Wire endpoint has no connection |
| label_dangling | ERROR | Label not connected to wire |
| duplicate_reference | ERROR | Same reference designator twice |
| endpoint_off_grid | WARNING | Not on 1.27mm grid |

### Running from Command Line

```bash
# Quick ERC check
python3 -c "
from kicad_lib import load_schematic, run_erc
sch = load_schematic('sdr_kicad/power_management.kicad_sch').unwrap()
erc = run_erc(sch)
print(erc.format_report())
"
```
