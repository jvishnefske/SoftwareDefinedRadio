"""Writer for KiCad symbol library files (.kicad_sym).

Serializes SymbolLibrary structures to S-expression format.
"""

from __future__ import annotations

from ..parser.sexpr import SExpr, SExprList
from ..types.primitives import Position, Stroke, Fill, Effects, Property
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
from .sexpr_writer import write_sexpr, format_number, make_sexpr


def _write_position(pos: Position, key: str = "at") -> SExprList:
    """Write a position as (at x y [angle])."""
    if pos.angle != 0:
        return make_sexpr(key, pos.x, pos.y, pos.angle)
    return make_sexpr(key, pos.x, pos.y)


def _write_stroke(stroke: Stroke) -> SExprList:
    """Write stroke as (stroke (width w) (type t) ...)."""
    result: SExprList = ["stroke"]
    result.append(make_sexpr("width", stroke.width))
    result.append(make_sexpr("type", stroke.type))
    if stroke.color:
        result.append(make_sexpr(
            "color",
            stroke.color.r,
            stroke.color.g,
            stroke.color.b,
            stroke.color.a,
        ))
    else:
        result.append(make_sexpr("color", 0, 0, 0, 0))
    return result


def _write_fill(fill: Fill) -> SExprList:
    """Write fill as (fill (type t))."""
    return make_sexpr("fill", make_sexpr("type", fill.type))


def _write_font(effects: Effects) -> SExprList:
    """Write font as (font (size x y) ...)."""
    font = effects.font
    result: SExprList = ["font"]
    result.append(make_sexpr("size", font.size_x, font.size_y))
    if font.bold:
        result.append("bold")
    if font.italic:
        result.append("italic")
    return result


def _write_effects(effects: Effects) -> SExprList:
    """Write effects as (effects (font ...) ...)."""
    result: SExprList = ["effects"]
    result.append(_write_font(effects))

    # Add justify if not default
    if effects.justify_h != "center" or effects.justify_v != "center":
        justify: SExprList = ["justify"]
        if effects.justify_h != "center":
            justify.append(effects.justify_h)
        if effects.justify_v != "center":
            justify.append(effects.justify_v)
        result.append(justify)

    if effects.hide:
        result.append("hide")

    return result


def _write_property(prop: Property) -> SExprList:
    """Write a property as (property name value ...)."""
    result: SExprList = ["property", prop.name, prop.value]
    result.append(make_sexpr("id", prop.id))
    result.append(_write_position(prop.position))
    result.append(_write_effects(prop.effects))
    return result


def _write_pin(pin: PinDef) -> SExprList:
    """Write a pin as (pin type shape ...)."""
    result: SExprList = ["pin", pin.electrical_type.value, pin.shape.value]
    result.append(_write_position(pin.position))
    result.append(make_sexpr("length", pin.length))

    # Name with effects
    name_expr: SExprList = ["name", pin.name]
    if pin.name_effects:
        name_expr.append(_write_effects(pin.name_effects))
    else:
        name_expr.append(make_sexpr("effects", make_sexpr("font", make_sexpr("size", 1.0, 1.0))))
    result.append(name_expr)

    # Number with effects
    number_expr: SExprList = ["number", pin.number]
    if pin.number_effects:
        number_expr.append(_write_effects(pin.number_effects))
    else:
        number_expr.append(make_sexpr("effects", make_sexpr("font", make_sexpr("size", 1.0, 1.0))))
    result.append(number_expr)

    if pin.hide:
        result.append("hide")

    return result


def _write_rectangle(rect: Rectangle) -> SExprList:
    """Write a rectangle as (rectangle (start ...) (end ...) ...)."""
    result: SExprList = ["rectangle"]
    result.append(make_sexpr("start", rect.start.x, rect.start.y))
    result.append(make_sexpr("end", rect.end.x, rect.end.y))
    result.append(_write_stroke(rect.stroke))
    result.append(_write_fill(rect.fill))
    return result


def _write_circle(circle: Circle) -> SExprList:
    """Write a circle as (circle (center ...) (radius ...) ...)."""
    result: SExprList = ["circle"]
    result.append(make_sexpr("center", circle.center.x, circle.center.y))
    result.append(make_sexpr("radius", circle.radius))
    result.append(_write_stroke(circle.stroke))
    result.append(_write_fill(circle.fill))
    return result


def _write_arc(arc: Arc) -> SExprList:
    """Write an arc as (arc (start ...) (mid ...) (end ...) ...)."""
    result: SExprList = ["arc"]
    result.append(make_sexpr("start", arc.start.x, arc.start.y))
    result.append(make_sexpr("mid", arc.mid.x, arc.mid.y))
    result.append(make_sexpr("end", arc.end.x, arc.end.y))
    result.append(_write_stroke(arc.stroke))
    result.append(_write_fill(arc.fill))
    return result


def _write_polyline(poly: Polyline) -> SExprList:
    """Write a polyline as (polyline (pts ...) ...)."""
    pts: SExprList = ["pts"]
    for pt in poly.points:
        pts.append(make_sexpr("xy", pt.x, pt.y))

    result: SExprList = ["polyline"]
    result.append(pts)
    result.append(_write_stroke(poly.stroke))
    result.append(_write_fill(poly.fill))
    return result


def _write_text(text: Text) -> SExprList:
    """Write text as (text "string" ...)."""
    result: SExprList = ["text", text.text]
    result.append(_write_position(text.position))
    if text.effects:
        result.append(_write_effects(text.effects))
    return result


def _write_graphic(graphic: GraphicElement) -> SExprList:
    """Write a graphic element."""
    if isinstance(graphic, Rectangle):
        return _write_rectangle(graphic)
    elif isinstance(graphic, Circle):
        return _write_circle(graphic)
    elif isinstance(graphic, Arc):
        return _write_arc(graphic)
    elif isinstance(graphic, Polyline):
        return _write_polyline(graphic)
    elif isinstance(graphic, Text):
        return _write_text(graphic)
    raise ValueError(f"Unknown graphic type: {type(graphic)}")


def _write_symbol_unit(unit: SymbolUnit) -> SExprList:
    """Write a symbol unit as (symbol name ...)."""
    result: SExprList = ["symbol", unit.name]

    for graphic in unit.graphics:
        result.append(_write_graphic(graphic))

    for pin in unit.pins:
        result.append(_write_pin(pin))

    return result


def _write_symbol_def(sym: SymbolDef) -> SExprList:
    """Write a symbol definition as (symbol name ...)."""
    result: SExprList = ["symbol", sym.name]

    # Flags
    result.append(make_sexpr("in_bom", "yes" if sym.in_bom else "no"))
    result.append(make_sexpr("on_board", "yes" if sym.on_board else "no"))

    if sym.extends:
        result.append(make_sexpr("extends", sym.extends))

    if sym.power:
        result.append(["power"])

    # Properties
    for prop in sym.properties:
        result.append(_write_property(prop))

    # Pin settings
    if sym.pin_numbers_hide:
        result.append(["pin_numbers", "hide"])
    if sym.pin_names_hide:
        result.append(["pin_names", "hide"])
    if sym.pin_names_offset != 0.508:
        result.append(make_sexpr("pin_names", make_sexpr("offset", sym.pin_names_offset)))

    # Units
    for unit in sym.units:
        result.append(_write_symbol_unit(unit))

    return result


def symbol_library_to_sexpr(lib: SymbolLibrary) -> SExprList:
    """Convert a SymbolLibrary to an S-expression.

    Args:
        lib: The symbol library to convert.

    Returns:
        The S-expression representation.
    """
    result: SExprList = ["kicad_symbol_lib"]
    result.append(make_sexpr("version", lib.version))
    result.append(make_sexpr("generator", lib.generator))

    for sym in lib.symbols:
        result.append(_write_symbol_def(sym))

    return result


def write_symbol_library(lib: SymbolLibrary) -> str:
    """Write a symbol library to a string.

    Args:
        lib: The symbol library to write.

    Returns:
        The formatted .kicad_sym file content.
    """
    sexpr = symbol_library_to_sexpr(lib)
    return write_sexpr(sexpr)
