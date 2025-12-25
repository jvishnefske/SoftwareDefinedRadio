"""Parser for KiCad symbol library files (.kicad_sym).

Converts S-expressions into typed SymbolLibrary structures.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from ..result import Result, Ok, Err
from ..types.primitives import Point, Position, Stroke, Fill, Effects, Font, Property
from ..types.enums import PinElectricalType, PinShape, FillType
from ..types.symbol import (
    PinDef,
    Rectangle,
    Circle,
    Arc,
    Polyline,
    Text,
    GraphicElement,
    SymbolUnit,
    SymbolDef,
    SymbolLibrary,
)
from .sexpr import (
    SExpr,
    SExprList,
    parse_sexpr,
    ParseError,
    sexpr_get,
    sexpr_get_all,
    sexpr_get_value,
    sexpr_get_float,
    sexpr_get_int,
    sexpr_has_flag,
)


@dataclass(frozen=True)
class SymbolParseError:
    """Error during symbol library parsing.

    Attributes:
        message: Description of the error.
        context: Additional context (e.g., symbol name).
    """

    message: str
    context: str = ""

    def __str__(self) -> str:
        if self.context:
            return f"{self.message} (in {self.context})"
        return self.message


def _parse_point(expr: SExprList) -> Point:
    """Parse a point from (xy x y) or positional args."""
    if len(expr) >= 3:
        x = float(expr[1]) if isinstance(expr[1], str) else 0.0
        y = float(expr[2]) if isinstance(expr[2], str) else 0.0
        return Point(x, y)
    return Point(0, 0)


def _parse_position(expr: SExpr) -> Position:
    """Parse position from (at x y [angle])."""
    at = sexpr_get(expr, "at")
    if at and isinstance(at, list) and len(at) >= 3:
        x = float(at[1]) if isinstance(at[1], str) else 0.0
        y = float(at[2]) if isinstance(at[2], str) else 0.0
        angle = float(at[3]) if len(at) > 3 and isinstance(at[3], str) else 0.0
        return Position(x, y, angle)
    return Position(0, 0)


def _parse_stroke(expr: SExpr) -> Stroke:
    """Parse stroke from (stroke (width w) (type t) ...)."""
    stroke_expr = sexpr_get(expr, "stroke")
    if not stroke_expr or not isinstance(stroke_expr, list):
        return Stroke(0)

    width = sexpr_get_float(stroke_expr, "width") or 0.0
    stroke_type = sexpr_get_value(stroke_expr, "type") or "default"

    return Stroke(width, stroke_type)


def _parse_fill(expr: SExpr) -> Fill:
    """Parse fill from (fill (type t) ...)."""
    fill_expr = sexpr_get(expr, "fill")
    if not fill_expr or not isinstance(fill_expr, list):
        return Fill("none")

    fill_type = sexpr_get_value(fill_expr, "type") or "none"
    return Fill(fill_type)


def _parse_font(expr: SExpr) -> Font:
    """Parse font from (font (size x y) ...)."""
    font_expr = sexpr_get(expr, "font")
    if not font_expr or not isinstance(font_expr, list):
        return Font()

    size = sexpr_get(font_expr, "size")
    size_x = 1.27
    size_y = 1.27
    if size and isinstance(size, list) and len(size) >= 3:
        size_x = float(size[1]) if isinstance(size[1], str) else 1.27
        size_y = float(size[2]) if isinstance(size[2], str) else 1.27

    bold = sexpr_has_flag(font_expr, "bold")
    italic = sexpr_has_flag(font_expr, "italic")

    return Font(size_x=size_x, size_y=size_y, bold=bold, italic=italic)


def _parse_effects(expr: SExpr) -> Effects:
    """Parse effects from (effects (font ...) (justify ...) ...)."""
    effects_expr = sexpr_get(expr, "effects")
    if not effects_expr or not isinstance(effects_expr, list):
        return Effects()

    font = _parse_font(effects_expr)

    justify_h = "center"
    justify_v = "center"
    justify = sexpr_get(effects_expr, "justify")
    if justify and isinstance(justify, list):
        for item in justify[1:]:
            if isinstance(item, str):
                if item in ("left", "right"):
                    justify_h = item
                elif item in ("top", "bottom"):
                    justify_v = item

    hide = sexpr_has_flag(effects_expr, "hide")

    return Effects(font, justify_h, justify_v, hide=hide)


def _parse_property(expr: SExprList) -> Property:
    """Parse a property from (property name value ...)."""
    name = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""
    value = expr[2] if len(expr) > 2 and isinstance(expr[2], str) else ""

    prop_id = sexpr_get_int(expr, "id") or 0
    position = _parse_position(expr)
    effects = _parse_effects(expr)

    return Property(name, value, prop_id, position, effects)


def _parse_pin(expr: SExprList) -> PinDef:
    """Parse a pin from (pin type shape (at ...) (length ...) (name ...) (number ...))."""
    # First two elements after 'pin' are type and shape
    electrical_type = PinElectricalType.UNSPECIFIED
    shape = PinShape.LINE

    idx = 1
    while idx < len(expr) and isinstance(expr[idx], str):
        val = expr[idx]
        # Check if it's an electrical type
        try:
            electrical_type = PinElectricalType.from_string(val)
            idx += 1
            continue
        except (KeyError, ValueError):
            pass
        # Check if it's a shape
        try:
            shape = PinShape.from_string(val)
            idx += 1
            continue
        except (KeyError, ValueError):
            pass
        idx += 1

    position = _parse_position(expr)

    length_expr = sexpr_get(expr, "length")
    length = 2.54
    if length_expr and isinstance(length_expr, list) and len(length_expr) >= 2:
        length = float(length_expr[1]) if isinstance(length_expr[1], str) else 2.54

    # Parse name
    name_expr = sexpr_get(expr, "name")
    name = ""
    name_effects = None
    if name_expr and isinstance(name_expr, list) and len(name_expr) >= 2:
        name = name_expr[1] if isinstance(name_expr[1], str) else ""
        name_effects = _parse_effects(name_expr)

    # Parse number
    number_expr = sexpr_get(expr, "number")
    number = ""
    number_effects = None
    if number_expr and isinstance(number_expr, list) and len(number_expr) >= 2:
        number = number_expr[1] if isinstance(number_expr[1], str) else ""
        number_effects = _parse_effects(number_expr)

    hide = sexpr_has_flag(expr, "hide")

    return PinDef(
        number=number,
        name=name,
        electrical_type=electrical_type,
        shape=shape,
        position=position,
        length=length,
        name_effects=name_effects,
        number_effects=number_effects,
        hide=hide,
    )


def _parse_rectangle(expr: SExprList) -> Rectangle:
    """Parse a rectangle from (rectangle (start ...) (end ...) ...)."""
    start = Point(0, 0)
    end = Point(0, 0)

    start_expr = sexpr_get(expr, "start")
    if start_expr and isinstance(start_expr, list) and len(start_expr) >= 3:
        start = Point(
            float(start_expr[1]) if isinstance(start_expr[1], str) else 0.0,
            float(start_expr[2]) if isinstance(start_expr[2], str) else 0.0,
        )

    end_expr = sexpr_get(expr, "end")
    if end_expr and isinstance(end_expr, list) and len(end_expr) >= 3:
        end = Point(
            float(end_expr[1]) if isinstance(end_expr[1], str) else 0.0,
            float(end_expr[2]) if isinstance(end_expr[2], str) else 0.0,
        )

    stroke = _parse_stroke(expr)
    fill = _parse_fill(expr)

    return Rectangle(start, end, stroke, fill)


def _parse_circle(expr: SExprList) -> Circle:
    """Parse a circle from (circle (center ...) (radius ...) ...)."""
    center = Point(0, 0)
    radius = 0.0

    center_expr = sexpr_get(expr, "center")
    if center_expr and isinstance(center_expr, list) and len(center_expr) >= 3:
        center = Point(
            float(center_expr[1]) if isinstance(center_expr[1], str) else 0.0,
            float(center_expr[2]) if isinstance(center_expr[2], str) else 0.0,
        )

    radius_expr = sexpr_get(expr, "radius")
    if radius_expr and isinstance(radius_expr, list) and len(radius_expr) >= 2:
        radius = float(radius_expr[1]) if isinstance(radius_expr[1], str) else 0.0

    stroke = _parse_stroke(expr)
    fill = _parse_fill(expr)

    return Circle(center, radius, stroke, fill)


def _parse_arc(expr: SExprList) -> Arc:
    """Parse an arc from (arc (start ...) (mid ...) (end ...) ...)."""
    start = Point(0, 0)
    mid = Point(0, 0)
    end = Point(0, 0)

    start_expr = sexpr_get(expr, "start")
    if start_expr and isinstance(start_expr, list) and len(start_expr) >= 3:
        start = Point(
            float(start_expr[1]) if isinstance(start_expr[1], str) else 0.0,
            float(start_expr[2]) if isinstance(start_expr[2], str) else 0.0,
        )

    mid_expr = sexpr_get(expr, "mid")
    if mid_expr and isinstance(mid_expr, list) and len(mid_expr) >= 3:
        mid = Point(
            float(mid_expr[1]) if isinstance(mid_expr[1], str) else 0.0,
            float(mid_expr[2]) if isinstance(mid_expr[2], str) else 0.0,
        )

    end_expr = sexpr_get(expr, "end")
    if end_expr and isinstance(end_expr, list) and len(end_expr) >= 3:
        end = Point(
            float(end_expr[1]) if isinstance(end_expr[1], str) else 0.0,
            float(end_expr[2]) if isinstance(end_expr[2], str) else 0.0,
        )

    stroke = _parse_stroke(expr)
    fill = _parse_fill(expr)

    return Arc(start, mid, end, stroke, fill)


def _parse_polyline(expr: SExprList) -> Polyline:
    """Parse a polyline from (polyline (pts (xy ...) ...) ...)."""
    points: list[Point] = []

    pts_expr = sexpr_get(expr, "pts")
    if pts_expr and isinstance(pts_expr, list):
        for item in pts_expr[1:]:
            if isinstance(item, list) and len(item) >= 3 and item[0] == "xy":
                x = float(item[1]) if isinstance(item[1], str) else 0.0
                y = float(item[2]) if isinstance(item[2], str) else 0.0
                points.append(Point(x, y))

    stroke = _parse_stroke(expr)
    fill = _parse_fill(expr)

    return Polyline(tuple(points), stroke, fill)


def _parse_text(expr: SExprList) -> Text:
    """Parse text from (text "string" (at ...) ...)."""
    text = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""
    position = _parse_position(expr)
    effects = _parse_effects(expr)

    return Text(text, position, effects)


def _parse_symbol_unit(expr: SExprList) -> SymbolUnit:
    """Parse a symbol unit from (symbol name ...)."""
    name = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""

    # Extract unit and style indices from name (e.g., "Symbol_1_1")
    unit_index = 1
    style_index = 1
    parts = name.rsplit("_", 2)
    if len(parts) >= 3:
        try:
            unit_index = int(parts[-2])
            style_index = int(parts[-1])
        except ValueError:
            pass

    pins: list[PinDef] = []
    graphics: list[GraphicElement] = []

    for item in expr[2:]:
        if not isinstance(item, list) or len(item) == 0:
            continue

        element_type = item[0]
        if element_type == "pin":
            pins.append(_parse_pin(item))
        elif element_type == "rectangle":
            graphics.append(_parse_rectangle(item))
        elif element_type == "circle":
            graphics.append(_parse_circle(item))
        elif element_type == "arc":
            graphics.append(_parse_arc(item))
        elif element_type == "polyline":
            graphics.append(_parse_polyline(item))
        elif element_type == "text":
            graphics.append(_parse_text(item))

    return SymbolUnit(
        name=name,
        unit_index=unit_index,
        style_index=style_index,
        pins=tuple(pins),
        graphics=tuple(graphics),
    )


def _parse_symbol_def(expr: SExprList) -> SymbolDef:
    """Parse a symbol definition from (symbol name ...)."""
    name = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""

    properties: list[Property] = []
    units: list[SymbolUnit] = []

    in_bom = True
    on_board = True
    extends = None
    power = False
    pin_numbers_hide = False
    pin_names_hide = False
    pin_names_offset = 0.508

    for item in expr[2:]:
        if not isinstance(item, list) or len(item) == 0:
            continue

        element_type = item[0]
        if element_type == "property":
            properties.append(_parse_property(item))
        elif element_type == "symbol":
            # Nested symbol is a unit
            units.append(_parse_symbol_unit(item))
        elif element_type == "in_bom":
            in_bom = item[1] == "yes" if len(item) > 1 else True
        elif element_type == "on_board":
            on_board = item[1] == "yes" if len(item) > 1 else True
        elif element_type == "extends":
            extends = item[1] if len(item) > 1 and isinstance(item[1], str) else None
        elif element_type == "power":
            power = True
        elif element_type == "pin_numbers":
            pin_numbers_hide = sexpr_has_flag(item, "hide")
        elif element_type == "pin_names":
            pin_names_hide = sexpr_has_flag(item, "hide")
            offset = sexpr_get_float(item, "offset")
            if offset is not None:
                pin_names_offset = offset

    return SymbolDef(
        name=name,
        properties=tuple(properties),
        units=tuple(units),
        in_bom=in_bom,
        on_board=on_board,
        extends=extends,
        power=power,
        pin_numbers_hide=pin_numbers_hide,
        pin_names_hide=pin_names_hide,
        pin_names_offset=pin_names_offset,
    )


def parse_symbol_library(text: str) -> Result[SymbolLibrary, SymbolParseError]:
    """Parse a symbol library from text content.

    Args:
        text: Contents of a .kicad_sym file.

    Returns:
        Ok(SymbolLibrary) on success, Err(SymbolParseError) on failure.
    """
    parse_result = parse_sexpr(text)
    if parse_result.is_err():
        err = parse_result.unwrap_err()
        return Err(SymbolParseError(str(err)))

    expr = parse_result.unwrap()

    if not isinstance(expr, list) or len(expr) == 0:
        return Err(SymbolParseError("Empty or invalid symbol library"))

    if expr[0] != "kicad_symbol_lib":
        return Err(SymbolParseError(f"Expected kicad_symbol_lib, got {expr[0]}"))

    version = sexpr_get_value(expr, "version") or "20210201"
    generator = sexpr_get_value(expr, "generator") or "unknown"

    symbols: list[SymbolDef] = []
    for sym_expr in sexpr_get_all(expr, "symbol"):
        symbols.append(_parse_symbol_def(sym_expr))

    return Ok(
        SymbolLibrary(
            version=version,
            generator=generator,
            symbols=tuple(symbols),
        )
    )


def load_symbol_library(path: Path | str) -> Result[SymbolLibrary, SymbolParseError]:
    """Load a symbol library from a file.

    Args:
        path: Path to the .kicad_sym file.

    Returns:
        Ok(SymbolLibrary) on success, Err(SymbolParseError) on failure.
    """
    path = Path(path)
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as e:
        return Err(SymbolParseError(f"Failed to read file: {e}", str(path)))

    return parse_symbol_library(text)
