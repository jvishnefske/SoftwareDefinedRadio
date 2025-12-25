"""Parser for KiCad schematic files (.kicad_sch).

Converts S-expressions into typed Schematic structures.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from ..result import Result, Ok, Err
from ..types.primitives import Point, Position, Stroke, Fill, Effects, Font, Property, UUID
from ..types.enums import LabelShape, SheetPinType
from ..types.schematic import (
    Wire,
    Bus,
    BusEntry,
    Junction,
    NoConnect,
    Label,
    GlobalLabel,
    HierarchicalLabel,
    PowerLabel,
    SymbolPin,
    SymbolInstance,
    SheetPin,
    Sheet,
    Text,
    SchematicPage,
    TitleBlock,
    LibSymbol,
    Schematic,
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
class SchematicParseError:
    """Error during schematic parsing.

    Attributes:
        message: Description of the error.
        context: Additional context.
    """

    message: str
    context: str = ""

    def __str__(self) -> str:
        if self.context:
            return f"{self.message} (in {self.context})"
        return self.message


def _parse_point(expr: SExprList) -> Point:
    """Parse a point from (xy x y)."""
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


def _parse_uuid(expr: SExpr) -> UUID:
    """Parse UUID from (uuid "...")."""
    uuid_val = sexpr_get_value(expr, "uuid")
    if uuid_val:
        return UUID(uuid_val)
    return UUID.generate()


def _parse_stroke(expr: SExpr) -> Stroke:
    """Parse stroke from (stroke ...)."""
    stroke_expr = sexpr_get(expr, "stroke")
    if not stroke_expr or not isinstance(stroke_expr, list):
        return Stroke(0)

    width = sexpr_get_float(stroke_expr, "width") or 0.0
    stroke_type = sexpr_get_value(stroke_expr, "type") or "default"

    return Stroke(width, stroke_type)


def _parse_font(expr: SExpr) -> Font:
    """Parse font from (font ...)."""
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
    """Parse effects from (effects ...)."""
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


def _parse_wire(expr: SExprList) -> Wire:
    """Parse a wire from (wire (pts ...) ...)."""
    start = Point(0, 0)
    end = Point(0, 0)

    pts = sexpr_get(expr, "pts")
    if pts and isinstance(pts, list):
        xy_list = [item for item in pts[1:] if isinstance(item, list) and item[0] == "xy"]
        if len(xy_list) >= 2:
            start = _parse_point(xy_list[0])
            end = _parse_point(xy_list[1])

    stroke = _parse_stroke(expr)
    uuid = _parse_uuid(expr)

    return Wire(start, end, stroke, uuid)


def _parse_bus(expr: SExprList) -> Bus:
    """Parse a bus from (bus (pts ...) ...)."""
    start = Point(0, 0)
    end = Point(0, 0)

    pts = sexpr_get(expr, "pts")
    if pts and isinstance(pts, list):
        xy_list = [item for item in pts[1:] if isinstance(item, list) and item[0] == "xy"]
        if len(xy_list) >= 2:
            start = _parse_point(xy_list[0])
            end = _parse_point(xy_list[1])

    stroke = _parse_stroke(expr)
    uuid = _parse_uuid(expr)

    return Bus(start, end, stroke, uuid)


def _parse_junction(expr: SExprList) -> Junction:
    """Parse a junction from (junction (at ...) ...)."""
    position = _parse_position(expr)
    diameter = sexpr_get_float(expr, "diameter") or 0.0
    uuid = _parse_uuid(expr)

    return Junction(position, diameter, uuid=uuid)


def _parse_no_connect(expr: SExprList) -> NoConnect:
    """Parse a no_connect from (no_connect (at ...) ...)."""
    position = _parse_position(expr)
    uuid = _parse_uuid(expr)

    return NoConnect(position, uuid)


def _parse_label(expr: SExprList) -> Label:
    """Parse a label from (label "text" (at ...) ...)."""
    text = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""
    position = _parse_position(expr)
    effects = _parse_effects(expr)
    uuid = _parse_uuid(expr)

    return Label(text, position, effects, uuid)


def _parse_global_label(expr: SExprList) -> GlobalLabel:
    """Parse a global_label from (global_label "text" ...)."""
    text = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""

    shape_val = sexpr_get_value(expr, "shape")
    shape = LabelShape.from_string(shape_val) if shape_val else LabelShape.PASSIVE

    position = _parse_position(expr)
    effects = _parse_effects(expr)
    uuid = _parse_uuid(expr)

    properties = []
    for prop_expr in sexpr_get_all(expr, "property"):
        properties.append(_parse_property(prop_expr))

    return GlobalLabel(text, shape, position, effects, uuid, tuple(properties))


def _parse_hierarchical_label(expr: SExprList) -> HierarchicalLabel:
    """Parse a hierarchical_label from (hierarchical_label "text" ...)."""
    text = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""

    shape_val = sexpr_get_value(expr, "shape")
    shape = LabelShape.from_string(shape_val) if shape_val else LabelShape.PASSIVE

    position = _parse_position(expr)
    effects = _parse_effects(expr)
    uuid = _parse_uuid(expr)

    return HierarchicalLabel(text, shape, position, effects, uuid)


def _parse_symbol_pin(expr: SExprList) -> SymbolPin:
    """Parse a pin reference from (pin "number" (uuid ...))."""
    number = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""
    uuid = _parse_uuid(expr)

    return SymbolPin(number, uuid)


def _parse_symbol_instance(expr: SExprList) -> SymbolInstance:
    """Parse a symbol instance from (symbol (lib_id ...) ...)."""
    lib_id = sexpr_get_value(expr, "lib_id") or ""
    position = _parse_position(expr)
    unit = sexpr_get_int(expr, "unit") or 1

    mirror_expr = sexpr_get(expr, "mirror")
    mirror = mirror_expr[1] if mirror_expr and len(mirror_expr) > 1 else ""

    in_bom = True
    in_bom_expr = sexpr_get(expr, "in_bom")
    if in_bom_expr and len(in_bom_expr) > 1:
        in_bom = in_bom_expr[1] == "yes"

    on_board = True
    on_board_expr = sexpr_get(expr, "on_board")
    if on_board_expr and len(on_board_expr) > 1:
        on_board = on_board_expr[1] == "yes"

    uuid = _parse_uuid(expr)

    properties = []
    for prop_expr in sexpr_get_all(expr, "property"):
        properties.append(_parse_property(prop_expr))

    pins = []
    for pin_expr in sexpr_get_all(expr, "pin"):
        pins.append(_parse_symbol_pin(pin_expr))

    return SymbolInstance(
        lib_id=lib_id,
        position=position,
        unit=unit,
        mirror=mirror,
        properties=tuple(properties),
        pins=tuple(pins),
        uuid=uuid,
        in_bom=in_bom,
        on_board=on_board,
    )


def _parse_sheet_pin(expr: SExprList) -> SheetPin:
    """Parse a sheet pin from (pin "name" ...)."""
    name = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""

    shape_val = expr[2] if len(expr) > 2 and isinstance(expr[2], str) else "passive"
    shape = SheetPinType.from_string(shape_val)

    position = _parse_position(expr)
    effects = _parse_effects(expr)
    uuid = _parse_uuid(expr)

    return SheetPin(name, shape, position, effects, uuid)


def _parse_sheet(expr: SExprList) -> Sheet:
    """Parse a sheet from (sheet (at ...) (size ...) ...)."""
    position = _parse_position(expr)

    size = Point(100, 100)
    size_expr = sexpr_get(expr, "size")
    if size_expr and isinstance(size_expr, list) and len(size_expr) >= 3:
        w = float(size_expr[1]) if isinstance(size_expr[1], str) else 100.0
        h = float(size_expr[2]) if isinstance(size_expr[2], str) else 100.0
        size = Point(w, h)

    stroke = _parse_stroke(expr)
    uuid = _parse_uuid(expr)

    properties = []
    for prop_expr in sexpr_get_all(expr, "property"):
        properties.append(_parse_property(prop_expr))

    pins = []
    for pin_expr in sexpr_get_all(expr, "pin"):
        pins.append(_parse_sheet_pin(pin_expr))

    return Sheet(
        position=position,
        size=size,
        properties=tuple(properties),
        pins=tuple(pins),
        stroke=stroke,
        uuid=uuid,
    )


def _parse_text(expr: SExprList) -> Text:
    """Parse text from (text "string" ...)."""
    text = expr[1] if len(expr) > 1 and isinstance(expr[1], str) else ""
    position = _parse_position(expr)
    effects = _parse_effects(expr)
    uuid = _parse_uuid(expr)

    return Text(text, position, effects, uuid)


def _parse_title_block(expr: SExprList) -> TitleBlock:
    """Parse title_block from (title_block ...)."""
    title = sexpr_get_value(expr, "title") or ""
    date = sexpr_get_value(expr, "date") or ""
    rev = sexpr_get_value(expr, "rev") or ""
    company = sexpr_get_value(expr, "company") or ""

    comments = ["", "", "", ""]
    for item in expr:
        if isinstance(item, list) and len(item) >= 3 and item[0] == "comment":
            idx = int(item[1]) if isinstance(item[1], str) else 0
            if 1 <= idx <= 4:
                comments[idx - 1] = item[2] if isinstance(item[2], str) else ""

    return TitleBlock(
        title=title,
        date=date,
        revision=rev,
        company=company,
        comment1=comments[0],
        comment2=comments[1],
        comment3=comments[2],
        comment4=comments[3],
    )


def _parse_page(expr: SExpr) -> SchematicPage:
    """Parse page/paper settings."""
    paper = sexpr_get_value(expr, "paper")
    if paper:
        return SchematicPage(size=paper)
    return SchematicPage()


def parse_schematic(text: str) -> Result[Schematic, SchematicParseError]:
    """Parse a schematic from text content.

    Args:
        text: Contents of a .kicad_sch file.

    Returns:
        Ok(Schematic) on success, Err(SchematicParseError) on failure.
    """
    parse_result = parse_sexpr(text)
    if parse_result.is_err():
        err = parse_result.unwrap_err()
        return Err(SchematicParseError(str(err)))

    expr = parse_result.unwrap()

    if not isinstance(expr, list) or len(expr) == 0:
        return Err(SchematicParseError("Empty or invalid schematic"))

    if expr[0] != "kicad_sch":
        return Err(SchematicParseError(f"Expected kicad_sch, got {expr[0]}"))

    version = sexpr_get_value(expr, "version") or "20231120"
    generator = sexpr_get_value(expr, "generator") or "unknown"
    uuid = _parse_uuid(expr)
    page = _parse_page(expr)

    title_block = TitleBlock()
    tb_expr = sexpr_get(expr, "title_block")
    if tb_expr and isinstance(tb_expr, list):
        title_block = _parse_title_block(tb_expr)

    wires: list[Wire] = []
    buses: list[Bus] = []
    junctions: list[Junction] = []
    no_connects: list[NoConnect] = []
    labels: list[Label] = []
    global_labels: list[GlobalLabel] = []
    hierarchical_labels: list[HierarchicalLabel] = []
    symbols: list[SymbolInstance] = []
    sheets: list[Sheet] = []
    texts: list[Text] = []

    for item in expr:
        if not isinstance(item, list) or len(item) == 0:
            continue

        element_type = item[0]
        if element_type == "wire":
            wires.append(_parse_wire(item))
        elif element_type == "bus":
            buses.append(_parse_bus(item))
        elif element_type == "junction":
            junctions.append(_parse_junction(item))
        elif element_type == "no_connect":
            no_connects.append(_parse_no_connect(item))
        elif element_type == "label":
            labels.append(_parse_label(item))
        elif element_type == "global_label":
            global_labels.append(_parse_global_label(item))
        elif element_type == "hierarchical_label":
            hierarchical_labels.append(_parse_hierarchical_label(item))
        elif element_type == "symbol":
            symbols.append(_parse_symbol_instance(item))
        elif element_type == "sheet":
            sheets.append(_parse_sheet(item))
        elif element_type == "text":
            texts.append(_parse_text(item))

    return Ok(
        Schematic(
            version=version,
            generator=generator,
            uuid=uuid,
            page=page,
            title_block=title_block,
            wires=tuple(wires),
            buses=tuple(buses),
            junctions=tuple(junctions),
            no_connects=tuple(no_connects),
            labels=tuple(labels),
            global_labels=tuple(global_labels),
            hierarchical_labels=tuple(hierarchical_labels),
            symbols=tuple(symbols),
            sheets=tuple(sheets),
            texts=tuple(texts),
        )
    )


def load_schematic(path: Path | str) -> Result[Schematic, SchematicParseError]:
    """Load a schematic from a file.

    Args:
        path: Path to the .kicad_sch file.

    Returns:
        Ok(Schematic) on success, Err(SchematicParseError) on failure.
    """
    path = Path(path)
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as e:
        return Err(SchematicParseError(f"Failed to read file: {e}", str(path)))

    return parse_schematic(text)
