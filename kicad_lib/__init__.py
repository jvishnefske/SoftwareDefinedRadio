"""KiCad file helper library.

Provides types, parsers, builders, and validators for working with
KiCad schematic, symbol, footprint, and project files.

Example usage:
    # Load and inspect a schematic
    from kicad_lib import load_schematic

    result = load_schematic("main.kicad_sch")
    if result.is_ok():
        sch = result.unwrap()
        print(f"Symbols: {len(sch.symbols)}")
        print(f"Wires: {len(sch.wires)}")

    # Create a symbol programmatically
    from kicad_lib import SymbolBuilder, write_symbol_library

    symbol = (SymbolBuilder("MyComponent")
        .with_reference("U")
        .with_footprint("Package_SO:SOIC-8")
        .add_body_rectangle()
        .add_pin_left("1", "VIN", 2.54, "power_in")
        .add_pin_right("2", "VOUT", 2.54, "power_out")
        .build())

    # Run ERC validation
    from kicad_lib import run_erc

    erc_result = run_erc(schematic)
    print(f"Errors: {erc_result.error_count}")
"""

# Result type
from .result import Result, Ok, Err, try_wrap

# Types - Primitives
from .types.primitives import (
    Point,
    Position,
    UUID,
    Color,
    Stroke,
    Fill,
    Font,
    Effects,
    Property,
)

# Types - Enums
from .types.enums import (
    PinElectricalType,
    PinShape,
    PinOrientation,
    LabelShape,
    Severity,
)

# Types - Symbol
from .types.symbol import (
    PinDef,
    Rectangle,
    Circle,
    Arc,
    Polyline,
    SymbolUnit,
    SymbolDef,
    SymbolLibrary,
)

# Types - Schematic
from .types.schematic import (
    Wire,
    Junction,
    NoConnect,
    Label,
    GlobalLabel,
    HierarchicalLabel,
    SymbolInstance,
    SheetPin,
    Sheet,
    SchematicPage,
    TitleBlock,
    Schematic,
)

# Parsers
from .parser import (
    parse_sexpr,
    parse_symbol_library,
    load_symbol_library,
    parse_schematic,
    load_schematic,
)

# Writers
from .writer import (
    write_sexpr,
    write_symbol_library,
)

# Builders
from .builders import (
    PinBuilder,
    SymbolBuilder,
    SymbolLibraryBuilder,
)

# Validation
from .validation import (
    ErcViolation,
    ErcResult,
    ErcRunner,
    run_erc,
)

__all__ = [
    # Result
    "Result",
    "Ok",
    "Err",
    "try_wrap",
    # Primitives
    "Point",
    "Position",
    "UUID",
    "Color",
    "Stroke",
    "Fill",
    "Font",
    "Effects",
    "Property",
    # Enums
    "PinElectricalType",
    "PinShape",
    "PinOrientation",
    "LabelShape",
    "Severity",
    # Symbol types
    "PinDef",
    "Rectangle",
    "Circle",
    "Arc",
    "Polyline",
    "SymbolUnit",
    "SymbolDef",
    "SymbolLibrary",
    # Schematic types
    "Wire",
    "Junction",
    "NoConnect",
    "Label",
    "GlobalLabel",
    "HierarchicalLabel",
    "SymbolInstance",
    "SheetPin",
    "Sheet",
    "SchematicPage",
    "TitleBlock",
    "Schematic",
    # Parsers
    "parse_sexpr",
    "parse_symbol_library",
    "load_symbol_library",
    "parse_schematic",
    "load_schematic",
    # Writers
    "write_sexpr",
    "write_symbol_library",
    # Builders
    "PinBuilder",
    "SymbolBuilder",
    "SymbolLibraryBuilder",
    # Validation
    "ErcViolation",
    "ErcResult",
    "ErcRunner",
    "run_erc",
]
