"""Symbol library types for KiCad .kicad_sym files.

Contains immutable dataclasses for representing symbol definitions,
pins, and graphical elements.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Sequence

from .primitives import Point, Position, Stroke, Fill, Effects, Property, UUID
from .enums import PinElectricalType, PinShape, PinOrientation, FillType


@dataclass(frozen=True)
class PinDef:
    """A symbol pin definition.

    Attributes:
        number: Pin number (as shown on schematic).
        name: Pin name/function.
        electrical_type: Electrical behavior for ERC.
        shape: Visual shape (line, inverted, clock, etc.).
        position: Pin location and rotation.
        length: Pin length in mm.
        name_effects: Text effects for pin name.
        number_effects: Text effects for pin number.
        hide: Whether pin is hidden.
    """

    number: str
    name: str
    electrical_type: PinElectricalType = PinElectricalType.UNSPECIFIED
    shape: PinShape = PinShape.LINE
    position: Position = field(default_factory=lambda: Position(0, 0))
    length: float = 2.54
    name_effects: Effects | None = None
    number_effects: Effects | None = None
    hide: bool = False

    @property
    def orientation(self) -> PinOrientation:
        """Get the pin orientation from its rotation angle."""
        angle = self.position.angle % 360
        if angle == 0:
            return PinOrientation.RIGHT
        elif angle == 90:
            return PinOrientation.UP
        elif angle == 180:
            return PinOrientation.LEFT
        else:  # 270
            return PinOrientation.DOWN

    def endpoint(self) -> Point:
        """Calculate the pin's endpoint (where wires connect).

        The endpoint is length away from position in the direction of orientation.
        """
        import math

        angle_rad = math.radians(self.position.angle)
        dx = self.length * math.cos(angle_rad)
        dy = self.length * math.sin(angle_rad)
        return Point(self.position.x + dx, self.position.y + dy)

    def with_position(self, position: Position) -> PinDef:
        """Returns a new PinDef with the specified position."""
        return PinDef(
            self.number,
            self.name,
            self.electrical_type,
            self.shape,
            position,
            self.length,
            self.name_effects,
            self.number_effects,
            self.hide,
        )

    def with_type(self, electrical_type: PinElectricalType) -> PinDef:
        """Returns a new PinDef with the specified electrical type."""
        return PinDef(
            self.number,
            self.name,
            electrical_type,
            self.shape,
            self.position,
            self.length,
            self.name_effects,
            self.number_effects,
            self.hide,
        )


@dataclass(frozen=True)
class Rectangle:
    """A rectangle graphic element.

    Attributes:
        start: Top-left corner.
        end: Bottom-right corner.
        stroke: Line stroke styling.
        fill: Fill styling.
    """

    start: Point
    end: Point
    stroke: Stroke = field(default_factory=lambda: Stroke(0))
    fill: Fill = field(default_factory=lambda: Fill("none"))

    @property
    def width(self) -> float:
        """Width of the rectangle."""
        return abs(self.end.x - self.start.x)

    @property
    def height(self) -> float:
        """Height of the rectangle."""
        return abs(self.end.y - self.start.y)

    @property
    def center(self) -> Point:
        """Center point of the rectangle."""
        return Point(
            (self.start.x + self.end.x) / 2, (self.start.y + self.end.y) / 2
        )


@dataclass(frozen=True)
class Circle:
    """A circle graphic element.

    Attributes:
        center: Center point.
        radius: Circle radius in mm.
        stroke: Line stroke styling.
        fill: Fill styling.
    """

    center: Point
    radius: float
    stroke: Stroke = field(default_factory=lambda: Stroke(0))
    fill: Fill = field(default_factory=lambda: Fill("none"))


@dataclass(frozen=True)
class Arc:
    """An arc graphic element.

    Attributes:
        start: Start point.
        mid: Midpoint on the arc.
        end: End point.
        stroke: Line stroke styling.
        fill: Fill styling.
    """

    start: Point
    mid: Point
    end: Point
    stroke: Stroke = field(default_factory=lambda: Stroke(0))
    fill: Fill = field(default_factory=lambda: Fill("none"))


@dataclass(frozen=True)
class Polyline:
    """A polyline graphic element (open or closed polygon).

    Attributes:
        points: Sequence of points defining the polyline.
        stroke: Line stroke styling.
        fill: Fill styling.
    """

    points: tuple[Point, ...]
    stroke: Stroke = field(default_factory=lambda: Stroke(0))
    fill: Fill = field(default_factory=lambda: Fill("none"))

    @classmethod
    def from_points(cls, points: Sequence[Point], stroke: Stroke | None = None, fill: Fill | None = None) -> Polyline:
        """Create a Polyline from a sequence of points."""
        return cls(
            tuple(points),
            stroke or Stroke(0),
            fill or Fill("none"),
        )


@dataclass(frozen=True)
class Text:
    """A text graphic element.

    Attributes:
        text: The text string.
        position: Text position and rotation.
        effects: Text display effects.
    """

    text: str
    position: Position = field(default_factory=lambda: Position(0, 0))
    effects: Effects | None = None


# Union type for all graphic elements
GraphicElement = Rectangle | Circle | Arc | Polyline | Text


@dataclass(frozen=True)
class SymbolUnit:
    """A unit within a multi-unit symbol.

    For single-unit symbols, there's typically one unit with index 1.
    Multi-unit symbols (like quad op-amps) have multiple units.

    Attributes:
        name: Unit name (e.g., "SymbolName_1_1").
        unit_index: Unit number (1-based).
        style_index: Style variant (usually 1).
        pins: Pins in this unit.
        graphics: Graphic elements in this unit.
    """

    name: str
    unit_index: int = 1
    style_index: int = 1
    pins: tuple[PinDef, ...] = ()
    graphics: tuple[GraphicElement, ...] = ()

    def with_pin(self, pin: PinDef) -> SymbolUnit:
        """Returns a new SymbolUnit with an added pin."""
        return SymbolUnit(
            self.name,
            self.unit_index,
            self.style_index,
            self.pins + (pin,),
            self.graphics,
        )

    def with_graphic(self, graphic: GraphicElement) -> SymbolUnit:
        """Returns a new SymbolUnit with an added graphic."""
        return SymbolUnit(
            self.name,
            self.unit_index,
            self.style_index,
            self.pins,
            self.graphics + (graphic,),
        )

    def pin_by_number(self, number: str) -> PinDef | None:
        """Find a pin by its number."""
        for pin in self.pins:
            if pin.number == number:
                return pin
        return None

    def pin_by_name(self, name: str) -> PinDef | None:
        """Find a pin by its name."""
        for pin in self.pins:
            if pin.name == name:
                return pin
        return None


@dataclass(frozen=True)
class SymbolDef:
    """A symbol definition in a symbol library.

    Attributes:
        name: Symbol name (library identifier).
        properties: Symbol properties (Reference, Value, Footprint, etc.).
        units: Symbol units containing pins and graphics.
        in_bom: Whether symbol appears in BOM.
        on_board: Whether symbol appears on PCB.
        extends: Name of parent symbol if this extends another.
        power: Whether this is a power symbol.
        pin_numbers_hide: Whether pin numbers are hidden.
        pin_names_hide: Whether pin names are hidden.
        pin_names_offset: Offset of pin names from pin.
    """

    name: str
    properties: tuple[Property, ...] = ()
    units: tuple[SymbolUnit, ...] = ()
    in_bom: bool = True
    on_board: bool = True
    extends: str | None = None
    power: bool = False
    pin_numbers_hide: bool = False
    pin_names_hide: bool = False
    pin_names_offset: float = 0.508

    @property
    def reference(self) -> str:
        """Get the reference designator prefix (e.g., 'U', 'R', 'C')."""
        for prop in self.properties:
            if prop.name == "Reference":
                return prop.value
        return "U"

    @property
    def value(self) -> str:
        """Get the symbol value."""
        for prop in self.properties:
            if prop.name == "Value":
                return prop.value
        return self.name

    @property
    def footprint(self) -> str | None:
        """Get the default footprint."""
        for prop in self.properties:
            if prop.name == "Footprint":
                return prop.value
        return None

    @property
    def datasheet(self) -> str | None:
        """Get the datasheet URL."""
        for prop in self.properties:
            if prop.name == "Datasheet":
                return prop.value
        return None

    def property_value(self, name: str) -> str | None:
        """Get a property value by name."""
        for prop in self.properties:
            if prop.name == name:
                return prop.value
        return None

    def all_pins(self) -> list[PinDef]:
        """Get all pins from all units."""
        pins = []
        for unit in self.units:
            pins.extend(unit.pins)
        return pins

    def pin_by_number(self, number: str) -> PinDef | None:
        """Find a pin by number across all units."""
        for unit in self.units:
            pin = unit.pin_by_number(number)
            if pin is not None:
                return pin
        return None

    def with_property(self, prop: Property) -> SymbolDef:
        """Returns a new SymbolDef with an added or updated property."""
        new_props = []
        found = False
        for p in self.properties:
            if p.name == prop.name:
                new_props.append(prop)
                found = True
            else:
                new_props.append(p)
        if not found:
            new_props.append(prop)
        return SymbolDef(
            self.name,
            tuple(new_props),
            self.units,
            self.in_bom,
            self.on_board,
            self.extends,
            self.power,
            self.pin_numbers_hide,
            self.pin_names_hide,
            self.pin_names_offset,
        )

    def with_unit(self, unit: SymbolUnit) -> SymbolDef:
        """Returns a new SymbolDef with an added unit."""
        return SymbolDef(
            self.name,
            self.properties,
            self.units + (unit,),
            self.in_bom,
            self.on_board,
            self.extends,
            self.power,
            self.pin_numbers_hide,
            self.pin_names_hide,
            self.pin_names_offset,
        )


@dataclass(frozen=True)
class SymbolLibrary:
    """A KiCad symbol library (.kicad_sym file).

    Attributes:
        version: File format version.
        generator: Tool that generated the file.
        symbols: Symbol definitions in the library.
    """

    version: str = "20210201"
    generator: str = "kicad_lib"
    symbols: tuple[SymbolDef, ...] = ()

    def symbol_by_name(self, name: str) -> SymbolDef | None:
        """Find a symbol by name."""
        for sym in self.symbols:
            if sym.name == name:
                return sym
        return None

    def with_symbol(self, symbol: SymbolDef) -> SymbolLibrary:
        """Returns a new SymbolLibrary with an added symbol."""
        return SymbolLibrary(
            self.version,
            self.generator,
            self.symbols + (symbol,),
        )

    @property
    def symbol_names(self) -> list[str]:
        """Get a list of all symbol names in the library."""
        return [sym.name for sym in self.symbols]
