"""Schematic types for KiCad .kicad_sch files.

Contains immutable dataclasses for representing schematic elements
including wires, symbols, labels, and hierarchical sheets.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Sequence

from .primitives import Point, Position, Stroke, Effects, Property, UUID
from .enums import LabelShape, SheetPinType


@dataclass(frozen=True)
class Wire:
    """A wire segment connecting two points.

    Attributes:
        start: Starting point of the wire.
        end: Ending point of the wire.
        stroke: Wire stroke styling.
        uuid: Unique identifier.
    """

    start: Point
    end: Point
    stroke: Stroke = field(default_factory=lambda: Stroke(0))
    uuid: UUID = field(default_factory=UUID.generate)

    @property
    def length(self) -> float:
        """Calculate wire length."""
        return self.start.distance_to(self.end)

    @property
    def is_horizontal(self) -> bool:
        """Check if wire is horizontal."""
        return abs(self.start.y - self.end.y) < 0.001

    @property
    def is_vertical(self) -> bool:
        """Check if wire is vertical."""
        return abs(self.start.x - self.end.x) < 0.001

    def contains_point(self, point: Point, tolerance: float = 0.01) -> bool:
        """Check if a point lies on this wire segment."""
        # Check if point is collinear and between endpoints
        d_total = self.start.distance_to(self.end)
        d_start = self.start.distance_to(point)
        d_end = self.end.distance_to(point)
        return abs(d_start + d_end - d_total) < tolerance


@dataclass(frozen=True)
class Bus:
    """A bus segment (multi-signal wire).

    Attributes:
        start: Starting point of the bus.
        end: Ending point of the bus.
        stroke: Bus stroke styling.
        uuid: Unique identifier.
    """

    start: Point
    end: Point
    stroke: Stroke = field(default_factory=lambda: Stroke(0))
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class BusEntry:
    """A bus entry connecting a wire to a bus.

    Attributes:
        position: Position of the entry.
        size: Size offset of the entry (dx, dy).
        stroke: Entry stroke styling.
        uuid: Unique identifier.
    """

    position: Position
    size: Point = field(default_factory=lambda: Point(2.54, 2.54))
    stroke: Stroke = field(default_factory=lambda: Stroke(0))
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class Junction:
    """A junction point where wires connect.

    Attributes:
        position: Junction location.
        diameter: Junction dot diameter.
        color: Junction color.
        uuid: Unique identifier.
    """

    position: Position
    diameter: float = 0.0
    color: tuple[int, int, int, float] = (0, 0, 0, 0.0)
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class NoConnect:
    """A no-connect marker on a pin.

    Attributes:
        position: Marker location.
        uuid: Unique identifier.
    """

    position: Position
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class Label:
    """A local net label.

    Attributes:
        text: Label text (net name).
        position: Label position and rotation.
        effects: Text display effects.
        uuid: Unique identifier.
    """

    text: str
    position: Position = field(default_factory=lambda: Position(0, 0))
    effects: Effects | None = None
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class GlobalLabel:
    """A global net label (visible across all sheets).

    Attributes:
        text: Label text (net name).
        shape: Label shape indicating signal direction.
        position: Label position and rotation.
        effects: Text display effects.
        uuid: Unique identifier.
        properties: Additional properties.
    """

    text: str
    shape: LabelShape = LabelShape.PASSIVE
    position: Position = field(default_factory=lambda: Position(0, 0))
    effects: Effects | None = None
    uuid: UUID = field(default_factory=UUID.generate)
    properties: tuple[Property, ...] = ()


@dataclass(frozen=True)
class HierarchicalLabel:
    """A hierarchical label connecting to a sheet pin.

    Attributes:
        text: Label text.
        shape: Label shape indicating signal direction.
        position: Label position and rotation.
        effects: Text display effects.
        uuid: Unique identifier.
    """

    text: str
    shape: LabelShape = LabelShape.PASSIVE
    position: Position = field(default_factory=lambda: Position(0, 0))
    effects: Effects | None = None
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class PowerLabel:
    """A power symbol label (e.g., VCC, GND).

    Attributes:
        text: Power net name.
        position: Label position and rotation.
        effects: Text display effects.
        uuid: Unique identifier.
    """

    text: str
    position: Position = field(default_factory=lambda: Position(0, 0))
    effects: Effects | None = None
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class SymbolPin:
    """A pin instance on a placed symbol.

    Attributes:
        number: Pin number.
        uuid: Unique identifier.
    """

    number: str
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class SymbolInstance:
    """A symbol placed on a schematic.

    Attributes:
        lib_id: Library symbol ID (e.g., "Device:R").
        position: Symbol position and rotation.
        unit: Unit number for multi-unit symbols.
        mirror: Mirror mode (x, y, or none).
        properties: Symbol properties (Reference, Value, etc.).
        pins: Pin instances.
        uuid: Unique identifier.
        in_bom: Whether symbol appears in BOM.
        on_board: Whether symbol appears on PCB.
    """

    lib_id: str
    position: Position = field(default_factory=lambda: Position(0, 0))
    unit: int = 1
    mirror: str = ""
    properties: tuple[Property, ...] = ()
    pins: tuple[SymbolPin, ...] = ()
    uuid: UUID = field(default_factory=UUID.generate)
    in_bom: bool = True
    on_board: bool = True

    @property
    def reference(self) -> str:
        """Get the reference designator."""
        for prop in self.properties:
            if prop.name == "Reference":
                return prop.value
        return "?"

    @property
    def value(self) -> str:
        """Get the symbol value."""
        for prop in self.properties:
            if prop.name == "Value":
                return prop.value
        return ""

    @property
    def footprint(self) -> str | None:
        """Get the footprint."""
        for prop in self.properties:
            if prop.name == "Footprint":
                return prop.value
        return None

    def property_value(self, name: str) -> str | None:
        """Get a property value by name."""
        for prop in self.properties:
            if prop.name == name:
                return prop.value
        return None

    def with_property(self, prop: Property) -> SymbolInstance:
        """Returns a new SymbolInstance with an added or updated property."""
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
        return SymbolInstance(
            self.lib_id,
            self.position,
            self.unit,
            self.mirror,
            tuple(new_props),
            self.pins,
            self.uuid,
            self.in_bom,
            self.on_board,
        )


@dataclass(frozen=True)
class SheetPin:
    """A pin on a hierarchical sheet.

    Attributes:
        name: Pin name (matches hierarchical label in subsheet).
        shape: Pin shape indicating signal direction.
        position: Pin position on the sheet rectangle.
        effects: Text display effects.
        uuid: Unique identifier.
    """

    name: str
    shape: SheetPinType = SheetPinType.PASSIVE
    position: Position = field(default_factory=lambda: Position(0, 0))
    effects: Effects | None = None
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class Sheet:
    """A hierarchical sheet reference.

    Attributes:
        position: Sheet position.
        size: Sheet rectangle size (width, height).
        properties: Sheet properties (name, file).
        pins: Sheet pins connecting to subsheet.
        stroke: Sheet border styling.
        fill: Sheet fill color.
        uuid: Unique identifier.
    """

    position: Position
    size: Point = field(default_factory=lambda: Point(100, 100))
    properties: tuple[Property, ...] = ()
    pins: tuple[SheetPin, ...] = ()
    stroke: Stroke = field(default_factory=lambda: Stroke(0.1524))
    fill: tuple[int, int, int, float] = (0, 0, 0, 0.0)
    uuid: UUID = field(default_factory=UUID.generate)

    @property
    def sheet_name(self) -> str:
        """Get the sheet name."""
        for prop in self.properties:
            if prop.name == "Sheetname":
                return prop.value
        return ""

    @property
    def file_name(self) -> str:
        """Get the sheet file name."""
        for prop in self.properties:
            if prop.name == "Sheetfile":
                return prop.value
        return ""


@dataclass(frozen=True)
class Text:
    """Free text annotation on a schematic.

    Attributes:
        text: Text content.
        position: Text position and rotation.
        effects: Text display effects.
        uuid: Unique identifier.
    """

    text: str
    position: Position = field(default_factory=lambda: Position(0, 0))
    effects: Effects | None = None
    uuid: UUID = field(default_factory=UUID.generate)


@dataclass(frozen=True)
class SchematicPage:
    """Page settings for a schematic.

    Attributes:
        size: Page size name (A4, A3, Letter, etc.) or custom.
        width: Custom width in mm.
        height: Custom height in mm.
        portrait: Whether page is portrait orientation.
    """

    size: str = "A4"
    width: float = 297.0
    height: float = 210.0
    portrait: bool = False


@dataclass(frozen=True)
class TitleBlock:
    """Title block information for a schematic.

    Attributes:
        title: Project title.
        date: Date string.
        revision: Revision string.
        company: Company name.
        comment1-4: Comment fields.
    """

    title: str = ""
    date: str = ""
    revision: str = ""
    company: str = ""
    comment1: str = ""
    comment2: str = ""
    comment3: str = ""
    comment4: str = ""


@dataclass(frozen=True)
class LibSymbol:
    """Cached library symbol embedded in schematic.

    This stores the symbol definition from a library for portability.
    """

    name: str
    content: object  # The parsed symbol definition


@dataclass(frozen=True)
class Schematic:
    """A KiCad schematic file (.kicad_sch).

    Attributes:
        version: File format version.
        generator: Tool that generated the file.
        uuid: Unique identifier.
        page: Page settings.
        title_block: Title block information.
        lib_symbols: Cached library symbols.
        wires: Wire segments.
        buses: Bus segments.
        bus_entries: Bus entry points.
        junctions: Junction points.
        no_connects: No-connect markers.
        labels: Local labels.
        global_labels: Global labels.
        hierarchical_labels: Hierarchical labels.
        power_labels: Power labels.
        symbols: Placed symbol instances.
        sheets: Hierarchical sheet references.
        texts: Text annotations.
    """

    version: str = "20231120"
    generator: str = "kicad_lib"
    uuid: UUID = field(default_factory=UUID.generate)
    page: SchematicPage = field(default_factory=SchematicPage)
    title_block: TitleBlock = field(default_factory=TitleBlock)
    lib_symbols: tuple[LibSymbol, ...] = ()
    wires: tuple[Wire, ...] = ()
    buses: tuple[Bus, ...] = ()
    bus_entries: tuple[BusEntry, ...] = ()
    junctions: tuple[Junction, ...] = ()
    no_connects: tuple[NoConnect, ...] = ()
    labels: tuple[Label, ...] = ()
    global_labels: tuple[GlobalLabel, ...] = ()
    hierarchical_labels: tuple[HierarchicalLabel, ...] = ()
    power_labels: tuple[PowerLabel, ...] = ()
    symbols: tuple[SymbolInstance, ...] = ()
    sheets: tuple[Sheet, ...] = ()
    texts: tuple[Text, ...] = ()

    def symbol_by_reference(self, reference: str) -> SymbolInstance | None:
        """Find a symbol by its reference designator."""
        for sym in self.symbols:
            if sym.reference == reference:
                return sym
        return None

    def symbols_by_lib_id(self, lib_id: str) -> list[SymbolInstance]:
        """Find all symbols with a given library ID."""
        return [sym for sym in self.symbols if sym.lib_id == lib_id]

    def sheet_by_name(self, name: str) -> Sheet | None:
        """Find a sheet by its name."""
        for sheet in self.sheets:
            if sheet.sheet_name == name:
                return sheet
        return None

    def with_wire(self, wire: Wire) -> Schematic:
        """Returns a new Schematic with an added wire."""
        return Schematic(
            self.version,
            self.generator,
            self.uuid,
            self.page,
            self.title_block,
            self.lib_symbols,
            self.wires + (wire,),
            self.buses,
            self.bus_entries,
            self.junctions,
            self.no_connects,
            self.labels,
            self.global_labels,
            self.hierarchical_labels,
            self.power_labels,
            self.symbols,
            self.sheets,
            self.texts,
        )

    def with_symbol(self, symbol: SymbolInstance) -> Schematic:
        """Returns a new Schematic with an added symbol."""
        return Schematic(
            self.version,
            self.generator,
            self.uuid,
            self.page,
            self.title_block,
            self.lib_symbols,
            self.wires,
            self.buses,
            self.bus_entries,
            self.junctions,
            self.no_connects,
            self.labels,
            self.global_labels,
            self.hierarchical_labels,
            self.power_labels,
            self.symbols + (symbol,),
            self.sheets,
            self.texts,
        )

    def with_label(self, label: Label) -> Schematic:
        """Returns a new Schematic with an added label."""
        return Schematic(
            self.version,
            self.generator,
            self.uuid,
            self.page,
            self.title_block,
            self.lib_symbols,
            self.wires,
            self.buses,
            self.bus_entries,
            self.junctions,
            self.no_connects,
            self.labels + (label,),
            self.global_labels,
            self.hierarchical_labels,
            self.power_labels,
            self.symbols,
            self.sheets,
            self.texts,
        )
