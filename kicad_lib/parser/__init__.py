"""KiCad file parsers.

Contains parsers for S-expression based KiCad files.
"""

from .symbol_parser import (
    SymbolParseError,
    parse_symbol_library,
    load_symbol_library,
)

from .schematic_parser import (
    SchematicParseError,
    parse_schematic,
    load_schematic,
)

from .sexpr import (
    ParseError,
    SExpr,
    SExprAtom,
    SExprList,
    Token,
    Tokenizer,
    Parser,
    parse_sexpr,
    parse_sexpr_file,
    sexpr_get,
    sexpr_get_all,
    sexpr_get_value,
    sexpr_get_float,
    sexpr_get_int,
    sexpr_has_flag,
)

__all__ = [
    # Symbol parser
    "SymbolParseError",
    "parse_symbol_library",
    "load_symbol_library",
    # Schematic parser
    "SchematicParseError",
    "parse_schematic",
    "load_schematic",
    # S-expression parser
    "ParseError",
    "SExpr",
    "SExprAtom",
    "SExprList",
    "Token",
    "Tokenizer",
    "Parser",
    "parse_sexpr",
    "parse_sexpr_file",
    "sexpr_get",
    "sexpr_get_all",
    "sexpr_get_value",
    "sexpr_get_float",
    "sexpr_get_int",
    "sexpr_has_flag",
]
