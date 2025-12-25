"""Builder for KiCad symbol creation.

Provides a fluent API for constructing symbol definitions with pins
and graphical elements.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Sequence

from ..types.primitives import Point, Position, Stroke, Fill, Effects, Font, Property
from ..types.enums import PinElectricalType, PinShape, PinOrientation
from ..types.symbol import (
    PinDef,
    Rectangle,
    Circle,
    Polyline,
    GraphicElement,
    SymbolUnit,
    SymbolDef,
    SymbolLibrary,
)


class PinBuilder:
    """Builder for creating pins with a fluent API."""

    def __init__(self, number: str, name: str):
        """Initialize the pin builder.

        Args:
            number: Pin number as shown on schematic.
            name: Pin name/function.
        """
        self._number = number
        self._name = name
        self._type = PinElectricalType.PASSIVE
        self._shape = PinShape.LINE
        self._x = 0.0
        self._y = 0.0
        self._angle = 0.0
        self._length = 2.54
        self._hide = False

    def at(self, x: float, y: float, angle: float = 0.0) -> PinBuilder:
        """Set pin position and angle.

        Args:
            x: X coordinate.
            y: Y coordinate.
            angle: Rotation angle in degrees (0=right, 90=up, 180=left, 270=down).
        """
        self._x = x
        self._y = y
        self._angle = angle
        return self

    def left(self, x: float, y: float) -> PinBuilder:
        """Place pin on left side of symbol (pointing right)."""
        return self.at(x, y, 0)

    def right(self, x: float, y: float) -> PinBuilder:
        """Place pin on right side of symbol (pointing left)."""
        return self.at(x, y, 180)

    def top(self, x: float, y: float) -> PinBuilder:
        """Place pin on top of symbol (pointing down)."""
        return self.at(x, y, 270)

    def bottom(self, x: float, y: float) -> PinBuilder:
        """Place pin on bottom of symbol (pointing up)."""
        return self.at(x, y, 90)

    def with_length(self, length: float) -> PinBuilder:
        """Set pin length."""
        self._length = length
        return self

    def input(self) -> PinBuilder:
        """Set electrical type to input."""
        self._type = PinElectricalType.INPUT
        return self

    def output(self) -> PinBuilder:
        """Set electrical type to output."""
        self._type = PinElectricalType.OUTPUT
        return self

    def bidirectional(self) -> PinBuilder:
        """Set electrical type to bidirectional."""
        self._type = PinElectricalType.BIDIRECTIONAL
        return self

    def passive(self) -> PinBuilder:
        """Set electrical type to passive."""
        self._type = PinElectricalType.PASSIVE
        return self

    def power_in(self) -> PinBuilder:
        """Set electrical type to power input."""
        self._type = PinElectricalType.POWER_IN
        return self

    def power_out(self) -> PinBuilder:
        """Set electrical type to power output."""
        self._type = PinElectricalType.POWER_OUT
        return self

    def no_connect(self) -> PinBuilder:
        """Set electrical type to no connect."""
        self._type = PinElectricalType.NO_CONNECT
        return self

    def with_type(self, pin_type: PinElectricalType) -> PinBuilder:
        """Set electrical type explicitly."""
        self._type = pin_type
        return self

    def inverted(self) -> PinBuilder:
        """Set pin shape to inverted (bubble)."""
        self._shape = PinShape.INVERTED
        return self

    def clock(self) -> PinBuilder:
        """Set pin shape to clock."""
        self._shape = PinShape.CLOCK
        return self

    def with_shape(self, shape: PinShape) -> PinBuilder:
        """Set pin shape explicitly."""
        self._shape = shape
        return self

    def hidden(self) -> PinBuilder:
        """Hide the pin."""
        self._hide = True
        return self

    def build(self) -> PinDef:
        """Build the pin definition."""
        return PinDef(
            number=self._number,
            name=self._name,
            electrical_type=self._type,
            shape=self._shape,
            position=Position(self._x, self._y, self._angle),
            length=self._length,
            hide=self._hide,
        )


class SymbolBuilder:
    """Builder for creating symbols with a fluent API."""

    def __init__(self, name: str):
        """Initialize the symbol builder.

        Args:
            name: Symbol name (library identifier).
        """
        self._name = name
        self._reference = "U"
        self._value = name
        self._footprint = ""
        self._datasheet = ""
        self._keywords = ""
        self._description = ""
        self._in_bom = True
        self._on_board = True
        self._power = False
        self._pin_names_hide = False
        self._pin_numbers_hide = False
        self._pins: list[PinDef] = []
        self._graphics: list[GraphicElement] = []
        self._custom_properties: list[tuple[str, str]] = []

    def with_reference(self, ref: str) -> SymbolBuilder:
        """Set the reference designator prefix (e.g., 'U', 'R', 'C')."""
        self._reference = ref
        return self

    def with_value(self, value: str) -> SymbolBuilder:
        """Set the symbol value."""
        self._value = value
        return self

    def with_footprint(self, footprint: str) -> SymbolBuilder:
        """Set the default footprint."""
        self._footprint = footprint
        return self

    def with_datasheet(self, url: str) -> SymbolBuilder:
        """Set the datasheet URL."""
        self._datasheet = url
        return self

    def with_keywords(self, keywords: str) -> SymbolBuilder:
        """Set keywords for searching."""
        self._keywords = keywords
        return self

    def with_description(self, description: str) -> SymbolBuilder:
        """Set the symbol description."""
        self._description = description
        return self

    def with_property(self, name: str, value: str) -> SymbolBuilder:
        """Add a custom property."""
        self._custom_properties.append((name, value))
        return self

    def exclude_from_bom(self) -> SymbolBuilder:
        """Exclude symbol from BOM."""
        self._in_bom = False
        return self

    def exclude_from_board(self) -> SymbolBuilder:
        """Exclude symbol from PCB."""
        self._on_board = False
        return self

    def as_power_symbol(self) -> SymbolBuilder:
        """Mark as a power symbol."""
        self._power = True
        self._reference = "#PWR"
        return self

    def hide_pin_names(self) -> SymbolBuilder:
        """Hide pin names."""
        self._pin_names_hide = True
        return self

    def hide_pin_numbers(self) -> SymbolBuilder:
        """Hide pin numbers."""
        self._pin_numbers_hide = True
        return self

    def add_pin(self, pin: PinDef) -> SymbolBuilder:
        """Add a pin to the symbol."""
        self._pins.append(pin)
        return self

    def add_pin_left(
        self,
        number: str,
        name: str,
        y_offset: float,
        pin_type: str = "passive",
        x: float = -10.16,
    ) -> SymbolBuilder:
        """Add a pin on the left side.

        Args:
            number: Pin number.
            name: Pin name.
            y_offset: Y offset from center.
            pin_type: Electrical type (passive, input, output, power_in, power_out, etc.).
            x: X position (negative = left of center).
        """
        pin = (
            PinBuilder(number, name)
            .left(x, y_offset)
            .with_type(PinElectricalType.from_string(pin_type))
            .build()
        )
        return self.add_pin(pin)

    def add_pin_right(
        self,
        number: str,
        name: str,
        y_offset: float,
        pin_type: str = "passive",
        x: float = 10.16,
    ) -> SymbolBuilder:
        """Add a pin on the right side.

        Args:
            number: Pin number.
            name: Pin name.
            y_offset: Y offset from center.
            pin_type: Electrical type.
            x: X position (positive = right of center).
        """
        pin = (
            PinBuilder(number, name)
            .right(x, y_offset)
            .with_type(PinElectricalType.from_string(pin_type))
            .build()
        )
        return self.add_pin(pin)

    def add_pin_top(
        self,
        number: str,
        name: str,
        x_offset: float,
        pin_type: str = "passive",
        y: float = 10.16,
    ) -> SymbolBuilder:
        """Add a pin on the top.

        Args:
            number: Pin number.
            name: Pin name.
            x_offset: X offset from center.
            pin_type: Electrical type.
            y: Y position (positive = above center).
        """
        pin = (
            PinBuilder(number, name)
            .top(x_offset, y)
            .with_type(PinElectricalType.from_string(pin_type))
            .build()
        )
        return self.add_pin(pin)

    def add_pin_bottom(
        self,
        number: str,
        name: str,
        x_offset: float,
        pin_type: str = "passive",
        y: float = -10.16,
    ) -> SymbolBuilder:
        """Add a pin on the bottom.

        Args:
            number: Pin number.
            name: Pin name.
            x_offset: X offset from center.
            pin_type: Electrical type.
            y: Y position (negative = below center).
        """
        pin = (
            PinBuilder(number, name)
            .bottom(x_offset, y)
            .with_type(PinElectricalType.from_string(pin_type))
            .build()
        )
        return self.add_pin(pin)

    def add_rectangle(
        self,
        x1: float,
        y1: float,
        x2: float,
        y2: float,
        fill_type: str = "background",
    ) -> SymbolBuilder:
        """Add a rectangle graphic.

        Args:
            x1, y1: First corner.
            x2, y2: Second corner.
            fill_type: Fill type (none, outline, background).
        """
        rect = Rectangle(
            start=Point(x1, y1),
            end=Point(x2, y2),
            stroke=Stroke(0),
            fill=Fill(fill_type),
        )
        self._graphics.append(rect)
        return self

    def add_body_rectangle(self, width: float = 15.24, height: float = 10.16) -> SymbolBuilder:
        """Add a centered body rectangle.

        Args:
            width: Rectangle width.
            height: Rectangle height.
        """
        return self.add_rectangle(
            -width / 2, height / 2,
            width / 2, -height / 2,
            "background",
        )

    def add_circle(
        self,
        x: float,
        y: float,
        radius: float,
        fill_type: str = "none",
    ) -> SymbolBuilder:
        """Add a circle graphic.

        Args:
            x, y: Center position.
            radius: Circle radius.
            fill_type: Fill type.
        """
        circle = Circle(
            center=Point(x, y),
            radius=radius,
            stroke=Stroke(0),
            fill=Fill(fill_type),
        )
        self._graphics.append(circle)
        return self

    def add_polyline(
        self,
        points: Sequence[tuple[float, float]],
        fill_type: str = "none",
    ) -> SymbolBuilder:
        """Add a polyline graphic.

        Args:
            points: Sequence of (x, y) tuples.
            fill_type: Fill type.
        """
        poly = Polyline(
            points=tuple(Point(x, y) for x, y in points),
            stroke=Stroke(0),
            fill=Fill(fill_type),
        )
        self._graphics.append(poly)
        return self

    def build(self) -> SymbolDef:
        """Build the symbol definition."""
        # Build properties
        props: list[Property] = []
        props.append(Property("Reference", self._reference, 0, Position(0, 2.54)))
        props.append(Property("Value", self._value, 1, Position(0, -2.54)))
        props.append(
            Property(
                "Footprint",
                self._footprint,
                2,
                Position(0, -5.08),
                Effects(hide=True),
            )
        )
        props.append(
            Property(
                "Datasheet",
                self._datasheet,
                3,
                Position(0, -7.62),
                Effects(hide=True),
            )
        )

        if self._keywords:
            props.append(
                Property(
                    "ki_keywords",
                    self._keywords,
                    4,
                    Position(0, 0),
                    Effects(hide=True),
                )
            )

        if self._description:
            props.append(
                Property(
                    "ki_description",
                    self._description,
                    5,
                    Position(0, 0),
                    Effects(hide=True),
                )
            )

        for i, (name, value) in enumerate(self._custom_properties):
            props.append(
                Property(name, value, 6 + i, Position(0, 0), Effects(hide=True))
            )

        # Build unit
        unit_name = f"{self._name}_0_1"
        unit = SymbolUnit(
            name=unit_name,
            unit_index=0,
            style_index=1,
            pins=tuple(self._pins),
            graphics=tuple(self._graphics),
        )

        return SymbolDef(
            name=self._name,
            properties=tuple(props),
            units=(unit,),
            in_bom=self._in_bom,
            on_board=self._on_board,
            power=self._power,
            pin_numbers_hide=self._pin_numbers_hide,
            pin_names_hide=self._pin_names_hide,
        )


class SymbolLibraryBuilder:
    """Builder for creating symbol libraries."""

    def __init__(self, version: str = "20210201", generator: str = "kicad_lib"):
        """Initialize the library builder.

        Args:
            version: File format version.
            generator: Generator identifier.
        """
        self._version = version
        self._generator = generator
        self._symbols: list[SymbolDef] = []

    def add_symbol(self, symbol: SymbolDef) -> SymbolLibraryBuilder:
        """Add a symbol to the library."""
        self._symbols.append(symbol)
        return self

    def build(self) -> SymbolLibrary:
        """Build the symbol library."""
        return SymbolLibrary(
            version=self._version,
            generator=self._generator,
            symbols=tuple(self._symbols),
        )
