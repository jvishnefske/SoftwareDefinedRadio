"""S-expression writer for KiCad files.

Serializes S-expression structures back to text format compatible with KiCad.
"""

from __future__ import annotations

from io import StringIO
from typing import Union, Sequence

from ..parser.sexpr import SExpr, SExprAtom, SExprList


def _needs_quoting(s: str) -> bool:
    """Check if a string needs to be quoted in S-expression output."""
    if not s:
        return True
    for ch in s:
        if ch in ' \t\n\r"()':
            return True
    return False


def _escape_string(s: str) -> str:
    """Escape a string for quoted output."""
    result = []
    for ch in s:
        if ch == '"':
            result.append('\\"')
        elif ch == '\\':
            result.append('\\\\')
        elif ch == '\n':
            result.append('\\n')
        elif ch == '\r':
            result.append('\\r')
        elif ch == '\t':
            result.append('\\t')
        else:
            result.append(ch)
    return ''.join(result)


def _format_atom(atom: str) -> str:
    """Format an atom for output."""
    if _needs_quoting(atom):
        return f'"{_escape_string(atom)}"'
    return atom


class SExprWriter:
    """Writer for S-expression output with configurable formatting."""

    def __init__(
        self,
        indent: str = "  ",
        max_inline_length: int = 80,
        compact_keys: set[str] | None = None,
    ):
        """Initialize the writer.

        Args:
            indent: String to use for each indentation level.
            max_inline_length: Maximum length for inline lists.
            compact_keys: Set of keys that should always be written inline.
        """
        self._indent = indent
        self._max_inline_length = max_inline_length
        self._compact_keys = compact_keys or {
            "at", "xy", "start", "end", "size", "stroke", "fill",
            "font", "justify", "color", "pts", "version", "generator",
        }

    def _should_inline(self, expr: SExpr, depth: int) -> bool:
        """Determine if an expression should be written inline."""
        if isinstance(expr, str):
            return True

        if len(expr) == 0:
            return True

        # Check if first element is a compact key
        if isinstance(expr[0], str) and expr[0] in self._compact_keys:
            return True

        # Simple key-value pairs are inline
        if len(expr) == 2 and all(isinstance(x, str) for x in expr):
            return True

        # Estimate length
        try:
            inline = self._write_inline(expr)
            if len(inline) <= self._max_inline_length:
                # Check if all children are simple
                if all(isinstance(x, str) for x in expr):
                    return True
                # Or if it's a simple nested list
                if len(expr) <= 3:
                    return True
        except Exception:
            pass

        return False

    def _write_inline(self, expr: SExpr) -> str:
        """Write an expression inline (without newlines)."""
        if isinstance(expr, str):
            return _format_atom(expr)

        parts = [self._write_inline(x) for x in expr]
        return "(" + " ".join(parts) + ")"

    def _write_expr(self, expr: SExpr, output: StringIO, depth: int) -> None:
        """Write an expression with proper formatting."""
        if isinstance(expr, str):
            output.write(_format_atom(expr))
            return

        if len(expr) == 0:
            output.write("()")
            return

        if self._should_inline(expr, depth):
            output.write(self._write_inline(expr))
            return

        # Multi-line formatting
        output.write("(")
        first = True
        for item in expr:
            if first:
                self._write_expr(item, output, depth)
                first = False
            else:
                output.write("\n")
                output.write(self._indent * (depth + 1))
                self._write_expr(item, output, depth + 1)
        output.write("\n")
        output.write(self._indent * depth)
        output.write(")")

    def write(self, expr: SExpr) -> str:
        """Write an S-expression to a string.

        Args:
            expr: The S-expression to write.

        Returns:
            The formatted string representation.
        """
        output = StringIO()
        self._write_expr(expr, output, 0)
        output.write("\n")
        return output.getvalue()


def write_sexpr(expr: SExpr, indent: str = "  ") -> str:
    """Write an S-expression to a string.

    Args:
        expr: The S-expression to write.
        indent: String to use for indentation.

    Returns:
        The formatted string representation.
    """
    writer = SExprWriter(indent=indent)
    return writer.write(expr)


def format_number(value: float) -> str:
    """Format a number for KiCad output.

    KiCad uses specific precision for different values.
    Removes unnecessary trailing zeros.
    """
    # Use enough precision for coordinates
    formatted = f"{value:.6f}"
    # Remove trailing zeros after decimal point
    if '.' in formatted:
        formatted = formatted.rstrip('0').rstrip('.')
    # Handle negative zero
    if formatted == '-0':
        formatted = '0'
    return formatted


def make_sexpr(*args: str | float | SExpr) -> SExprList:
    """Create an S-expression list from arguments.

    Floats are converted to formatted strings.

    Args:
        *args: Elements of the S-expression.

    Returns:
        An S-expression list.
    """
    result: SExprList = []
    for arg in args:
        if isinstance(arg, float):
            result.append(format_number(arg))
        elif isinstance(arg, int):
            result.append(str(arg))
        elif isinstance(arg, list):
            result.append(arg)
        else:
            result.append(arg)
    return result
