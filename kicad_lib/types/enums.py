"""Enumeration types for KiCad file representation.

Contains enum classes for pin types, shapes, fill modes, and other
categorical values used in KiCad schematic and symbol files.
"""

from enum import Enum, auto


class PinElectricalType(Enum):
    """Electrical type of a symbol pin.

    Determines how the pin behaves in ERC (Electrical Rules Check).
    """

    INPUT = "input"
    OUTPUT = "output"
    BIDIRECTIONAL = "bidirectional"
    TRI_STATE = "tri_state"
    PASSIVE = "passive"
    FREE = "free"
    UNSPECIFIED = "unspecified"
    POWER_IN = "power_in"
    POWER_OUT = "power_out"
    OPEN_COLLECTOR = "open_collector"
    OPEN_EMITTER = "open_emitter"
    NO_CONNECT = "no_connect"

    @classmethod
    def from_string(cls, s: str) -> "PinElectricalType":
        """Creates a PinElectricalType from a KiCad string."""
        mapping = {
            "input": cls.INPUT,
            "output": cls.OUTPUT,
            "bidirectional": cls.BIDIRECTIONAL,
            "tri_state": cls.TRI_STATE,
            "passive": cls.PASSIVE,
            "free": cls.FREE,
            "unspecified": cls.UNSPECIFIED,
            "power_in": cls.POWER_IN,
            "power_out": cls.POWER_OUT,
            "open_collector": cls.OPEN_COLLECTOR,
            "open_emitter": cls.OPEN_EMITTER,
            "no_connect": cls.NO_CONNECT,
        }
        return mapping.get(s.lower(), cls.UNSPECIFIED)


class PinShape(Enum):
    """Visual shape of a symbol pin."""

    LINE = "line"
    INVERTED = "inverted"
    CLOCK = "clock"
    INVERTED_CLOCK = "inverted_clock"
    INPUT_LOW = "input_low"
    CLOCK_LOW = "clock_low"
    OUTPUT_LOW = "output_low"
    EDGE_CLOCK_HIGH = "edge_clock_high"
    NON_LOGIC = "non_logic"

    @classmethod
    def from_string(cls, s: str) -> "PinShape":
        """Creates a PinShape from a KiCad string."""
        mapping = {
            "line": cls.LINE,
            "inverted": cls.INVERTED,
            "clock": cls.CLOCK,
            "inverted_clock": cls.INVERTED_CLOCK,
            "input_low": cls.INPUT_LOW,
            "clock_low": cls.CLOCK_LOW,
            "output_low": cls.OUTPUT_LOW,
            "edge_clock_high": cls.EDGE_CLOCK_HIGH,
            "non_logic": cls.NON_LOGIC,
        }
        return mapping.get(s.lower(), cls.LINE)


class PinOrientation(Enum):
    """Direction a pin points (towards the symbol body)."""

    RIGHT = "R"  # Pin on left side, pointing right
    LEFT = "L"  # Pin on right side, pointing left
    UP = "U"  # Pin on bottom, pointing up
    DOWN = "D"  # Pin on top, pointing down

    @classmethod
    def from_string(cls, s: str) -> "PinOrientation":
        """Creates a PinOrientation from a KiCad string."""
        mapping = {"R": cls.RIGHT, "L": cls.LEFT, "U": cls.UP, "D": cls.DOWN}
        return mapping.get(s.upper(), cls.RIGHT)


class FillType(Enum):
    """Fill type for graphic shapes."""

    NONE = "none"
    OUTLINE = "outline"
    BACKGROUND = "background"
    COLOR = "color"

    @classmethod
    def from_string(cls, s: str) -> "FillType":
        """Creates a FillType from a KiCad string."""
        mapping = {
            "none": cls.NONE,
            "outline": cls.OUTLINE,
            "background": cls.BACKGROUND,
            "color": cls.COLOR,
        }
        return mapping.get(s.lower(), cls.NONE)


class StrokeType(Enum):
    """Line stroke pattern type."""

    DEFAULT = "default"
    SOLID = "solid"
    DASH = "dash"
    DOT = "dot"
    DASH_DOT = "dash_dot"
    DASH_DOT_DOT = "dash_dot_dot"

    @classmethod
    def from_string(cls, s: str) -> "StrokeType":
        """Creates a StrokeType from a KiCad string."""
        mapping = {
            "default": cls.DEFAULT,
            "solid": cls.SOLID,
            "dash": cls.DASH,
            "dot": cls.DOT,
            "dash_dot": cls.DASH_DOT,
            "dash_dot_dot": cls.DASH_DOT_DOT,
        }
        return mapping.get(s.lower(), cls.DEFAULT)


class LabelShape(Enum):
    """Shape of hierarchical labels and global labels."""

    INPUT = "input"
    OUTPUT = "output"
    BIDIRECTIONAL = "bidirectional"
    TRI_STATE = "tri_state"
    PASSIVE = "passive"

    @classmethod
    def from_string(cls, s: str) -> "LabelShape":
        """Creates a LabelShape from a KiCad string."""
        mapping = {
            "input": cls.INPUT,
            "output": cls.OUTPUT,
            "bidirectional": cls.BIDIRECTIONAL,
            "tri_state": cls.TRI_STATE,
            "passive": cls.PASSIVE,
        }
        return mapping.get(s.lower(), cls.PASSIVE)


class JustifyHorizontal(Enum):
    """Horizontal text justification."""

    LEFT = "left"
    CENTER = "center"
    RIGHT = "right"

    @classmethod
    def from_string(cls, s: str) -> "JustifyHorizontal":
        """Creates a JustifyHorizontal from a KiCad string."""
        mapping = {"left": cls.LEFT, "center": cls.CENTER, "right": cls.RIGHT}
        return mapping.get(s.lower(), cls.CENTER)


class JustifyVertical(Enum):
    """Vertical text justification."""

    TOP = "top"
    CENTER = "center"
    BOTTOM = "bottom"

    @classmethod
    def from_string(cls, s: str) -> "JustifyVertical":
        """Creates a JustifyVertical from a KiCad string."""
        mapping = {"top": cls.TOP, "center": cls.CENTER, "bottom": cls.BOTTOM}
        return mapping.get(s.lower(), cls.CENTER)


class Severity(Enum):
    """Severity level for ERC violations."""

    ERROR = "error"
    WARNING = "warning"
    INFO = "info"
    IGNORE = "ignore"

    @classmethod
    def from_string(cls, s: str) -> "Severity":
        """Creates a Severity from a string."""
        mapping = {
            "error": cls.ERROR,
            "warning": cls.WARNING,
            "info": cls.INFO,
            "ignore": cls.IGNORE,
        }
        return mapping.get(s.lower(), cls.WARNING)


class PowerFlag(Enum):
    """Power flag type for power symbols."""

    POWER = "power"
    GROUND = "ground"

    @classmethod
    def from_string(cls, s: str) -> "PowerFlag":
        """Creates a PowerFlag from a KiCad string."""
        if "ground" in s.lower() or "gnd" in s.lower():
            return cls.GROUND
        return cls.POWER


class SheetPinType(Enum):
    """Type of sheet pin connection."""

    INPUT = "input"
    OUTPUT = "output"
    BIDIRECTIONAL = "bidirectional"
    TRI_STATE = "tri_state"
    PASSIVE = "passive"

    @classmethod
    def from_string(cls, s: str) -> "SheetPinType":
        """Creates a SheetPinType from a KiCad string."""
        mapping = {
            "input": cls.INPUT,
            "output": cls.OUTPUT,
            "bidirectional": cls.BIDIRECTIONAL,
            "tri_state": cls.TRI_STATE,
            "passive": cls.PASSIVE,
        }
        return mapping.get(s.lower(), cls.PASSIVE)


class SymbolUnit(Enum):
    """Symbol unit type for multi-unit symbols."""

    UNIT_A = 1
    UNIT_B = 2
    UNIT_C = 3
    UNIT_D = 4
    UNIT_E = 5
    UNIT_F = 6
    UNIT_G = 7
    UNIT_H = 8


# Pin compatibility matrix for ERC
# Key: (pin1_type, pin2_type), Value: Severity
PIN_COMPATIBILITY: dict[tuple[PinElectricalType, PinElectricalType], Severity] = {
    # Output to output is always an error
    (PinElectricalType.OUTPUT, PinElectricalType.OUTPUT): Severity.ERROR,
    (PinElectricalType.OUTPUT, PinElectricalType.POWER_OUT): Severity.ERROR,
    (PinElectricalType.POWER_OUT, PinElectricalType.OUTPUT): Severity.ERROR,
    (PinElectricalType.POWER_OUT, PinElectricalType.POWER_OUT): Severity.ERROR,
    # Power input to output is an error
    (PinElectricalType.POWER_IN, PinElectricalType.OUTPUT): Severity.ERROR,
    (PinElectricalType.OUTPUT, PinElectricalType.POWER_IN): Severity.ERROR,
    # Input to input is a warning
    (PinElectricalType.INPUT, PinElectricalType.INPUT): Severity.WARNING,
    # Power in to power in is OK (they can be bussed)
    (PinElectricalType.POWER_IN, PinElectricalType.POWER_IN): Severity.IGNORE,
}


def check_pin_compatibility(
    type1: PinElectricalType, type2: PinElectricalType
) -> Severity:
    """Check the compatibility of two connected pin types.

    Args:
        type1: Electrical type of the first pin.
        type2: Electrical type of the second pin.

    Returns:
        Severity level of connecting these pin types.
    """
    # Check both orderings since matrix may not be symmetric
    key = (type1, type2)
    if key in PIN_COMPATIBILITY:
        return PIN_COMPATIBILITY[key]

    key_rev = (type2, type1)
    if key_rev in PIN_COMPATIBILITY:
        return PIN_COMPATIBILITY[key_rev]

    # Default: connection is OK
    return Severity.IGNORE
