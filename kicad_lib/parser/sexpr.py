"""S-expression parser for KiCad files.

Provides tokenization and parsing of KiCad's Lisp-like S-expression format
used in .kicad_sch, .kicad_sym, .kicad_mod, and table files.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Union, Iterator
import re

from ..result import Result, Ok, Err


@dataclass(frozen=True)
class ParseError:
    """Error during S-expression parsing.

    Attributes:
        message: Description of the error.
        line: Line number where error occurred (1-indexed).
        column: Column number where error occurred (1-indexed).
    """

    message: str
    line: int
    column: int

    def __str__(self) -> str:
        return f"Parse error at line {self.line}, column {self.column}: {self.message}"


# S-expression value types
SExprAtom = str
SExprList = list["SExpr"]
SExpr = Union[SExprAtom, SExprList]


@dataclass(frozen=True)
class Token:
    """A lexical token from S-expression input.

    Attributes:
        type: Token type (LPAREN, RPAREN, STRING, ATOM).
        value: Token value.
        line: Line number (1-indexed).
        column: Column number (1-indexed).
    """

    type: str
    value: str
    line: int
    column: int


class Tokenizer:
    """Tokenizes S-expression input into tokens."""

    def __init__(self, text: str):
        """Initialize tokenizer with input text.

        Args:
            text: The S-expression text to tokenize.
        """
        self._text = text
        self._pos = 0
        self._line = 1
        self._col = 1

    def _advance(self) -> str:
        """Advance position and return current character."""
        ch = self._text[self._pos]
        self._pos += 1
        if ch == "\n":
            self._line += 1
            self._col = 1
        else:
            self._col += 1
        return ch

    def _peek(self) -> str:
        """Return current character without advancing."""
        if self._pos >= len(self._text):
            return ""
        return self._text[self._pos]

    def _skip_whitespace(self) -> None:
        """Skip whitespace and comments."""
        while self._pos < len(self._text):
            ch = self._peek()
            if ch in " \t\n\r":
                self._advance()
            elif ch == ";":
                # Skip comment to end of line
                while self._pos < len(self._text) and self._peek() != "\n":
                    self._advance()
            else:
                break

    def _read_string(self) -> Result[Token, ParseError]:
        """Read a quoted string token."""
        start_line = self._line
        start_col = self._col
        self._advance()  # Skip opening quote

        chars: list[str] = []
        while self._pos < len(self._text):
            ch = self._peek()
            if ch == '"':
                self._advance()  # Skip closing quote
                return Ok(Token("STRING", "".join(chars), start_line, start_col))
            elif ch == "\\":
                self._advance()  # Skip backslash
                if self._pos < len(self._text):
                    escape_ch = self._advance()
                    if escape_ch == "n":
                        chars.append("\n")
                    elif escape_ch == "t":
                        chars.append("\t")
                    elif escape_ch == "r":
                        chars.append("\r")
                    elif escape_ch == "\\":
                        chars.append("\\")
                    elif escape_ch == '"':
                        chars.append('"')
                    else:
                        chars.append(escape_ch)
            else:
                chars.append(self._advance())

        return Err(ParseError("Unterminated string", start_line, start_col))

    def _read_atom(self) -> Token:
        """Read an unquoted atom token."""
        start_line = self._line
        start_col = self._col
        chars: list[str] = []

        while self._pos < len(self._text):
            ch = self._peek()
            if ch in " \t\n\r()\"":
                break
            chars.append(self._advance())

        return Token("ATOM", "".join(chars), start_line, start_col)

    def tokenize(self) -> Result[list[Token], ParseError]:
        """Tokenize the entire input.

        Returns:
            Ok(list of tokens) on success, Err(ParseError) on failure.
        """
        tokens: list[Token] = []

        while self._pos < len(self._text):
            self._skip_whitespace()
            if self._pos >= len(self._text):
                break

            ch = self._peek()
            line, col = self._line, self._col

            if ch == "(":
                self._advance()
                tokens.append(Token("LPAREN", "(", line, col))
            elif ch == ")":
                self._advance()
                tokens.append(Token("RPAREN", ")", line, col))
            elif ch == '"':
                result = self._read_string()
                if result.is_err():
                    return result
                tokens.append(result.unwrap())
            else:
                tokens.append(self._read_atom())

        return Ok(tokens)


class Parser:
    """Parses a token stream into S-expressions."""

    def __init__(self, tokens: list[Token]):
        """Initialize parser with token list.

        Args:
            tokens: List of tokens from tokenizer.
        """
        self._tokens = tokens
        self._pos = 0

    def _peek(self) -> Token | None:
        """Return current token without advancing."""
        if self._pos >= len(self._tokens):
            return None
        return self._tokens[self._pos]

    def _advance(self) -> Token | None:
        """Advance position and return current token."""
        token = self._peek()
        self._pos += 1
        return token

    def _parse_list(self, start_token: Token) -> Result[SExprList, ParseError]:
        """Parse a list starting after the opening paren."""
        items: SExprList = []

        while True:
            token = self._peek()
            if token is None:
                return Err(
                    ParseError(
                        "Unexpected end of input in list",
                        start_token.line,
                        start_token.column,
                    )
                )

            if token.type == "RPAREN":
                self._advance()  # Consume closing paren
                return Ok(items)

            result = self._parse_expr()
            if result.is_err():
                return result
            items.append(result.unwrap())

    def _parse_expr(self) -> Result[SExpr, ParseError]:
        """Parse a single S-expression."""
        token = self._peek()
        if token is None:
            return Err(ParseError("Unexpected end of input", 0, 0))

        if token.type == "LPAREN":
            self._advance()  # Consume opening paren
            return self._parse_list(token)
        elif token.type == "RPAREN":
            return Err(
                ParseError("Unexpected closing paren", token.line, token.column)
            )
        else:
            self._advance()
            return Ok(token.value)

    def parse(self) -> Result[SExpr, ParseError]:
        """Parse the token stream into an S-expression.

        Returns:
            Ok(SExpr) on success, Err(ParseError) on failure.
        """
        result = self._parse_expr()
        if result.is_err():
            return result

        # Check for trailing content
        if self._pos < len(self._tokens):
            token = self._tokens[self._pos]
            return Err(
                ParseError(
                    f"Unexpected token after expression: {token.value}",
                    token.line,
                    token.column,
                )
            )

        return result

    def parse_all(self) -> Result[list[SExpr], ParseError]:
        """Parse all S-expressions from the token stream.

        Returns:
            Ok(list of SExpr) on success, Err(ParseError) on failure.
        """
        exprs: list[SExpr] = []

        while self._pos < len(self._tokens):
            result = self._parse_expr()
            if result.is_err():
                return result
            exprs.append(result.unwrap())

        return Ok(exprs)


def parse_sexpr(text: str) -> Result[SExpr, ParseError]:
    """Parse a string containing a single S-expression.

    Args:
        text: S-expression text to parse.

    Returns:
        Ok(SExpr) on success, Err(ParseError) on failure.
    """
    tokenizer = Tokenizer(text)
    tokens_result = tokenizer.tokenize()
    if tokens_result.is_err():
        return tokens_result

    parser = Parser(tokens_result.unwrap())
    return parser.parse()


def parse_sexpr_file(text: str) -> Result[list[SExpr], ParseError]:
    """Parse a string containing multiple S-expressions.

    Args:
        text: S-expression text to parse.

    Returns:
        Ok(list of SExpr) on success, Err(ParseError) on failure.
    """
    tokenizer = Tokenizer(text)
    tokens_result = tokenizer.tokenize()
    if tokens_result.is_err():
        return Err(tokens_result.unwrap_err())

    parser = Parser(tokens_result.unwrap())
    return parser.parse_all()


# Helper functions for working with S-expressions


def sexpr_get(expr: SExpr, key: str) -> SExpr | None:
    """Get the first child list starting with the given key.

    Args:
        expr: The S-expression to search.
        key: The key to search for.

    Returns:
        The matching child list, or None if not found.
    """
    if not isinstance(expr, list):
        return None

    for item in expr:
        if isinstance(item, list) and len(item) > 0 and item[0] == key:
            return item

    return None


def sexpr_get_all(expr: SExpr, key: str) -> list[SExprList]:
    """Get all child lists starting with the given key.

    Args:
        expr: The S-expression to search.
        key: The key to search for.

    Returns:
        List of matching child lists.
    """
    if not isinstance(expr, list):
        return []

    results: list[SExprList] = []
    for item in expr:
        if isinstance(item, list) and len(item) > 0 and item[0] == key:
            results.append(item)

    return results


def sexpr_get_value(expr: SExpr, key: str) -> str | None:
    """Get the value of a key-value pair.

    For an expression like (key value), returns value.

    Args:
        expr: The S-expression to search.
        key: The key to search for.

    Returns:
        The value string, or None if not found.
    """
    child = sexpr_get(expr, key)
    if child is not None and isinstance(child, list) and len(child) >= 2:
        val = child[1]
        if isinstance(val, str):
            return val
    return None


def sexpr_get_float(expr: SExpr, key: str) -> float | None:
    """Get a float value from a key-value pair.

    Args:
        expr: The S-expression to search.
        key: The key to search for.

    Returns:
        The float value, or None if not found or invalid.
    """
    val = sexpr_get_value(expr, key)
    if val is not None:
        try:
            return float(val)
        except ValueError:
            pass
    return None


def sexpr_get_int(expr: SExpr, key: str) -> int | None:
    """Get an integer value from a key-value pair.

    Args:
        expr: The S-expression to search.
        key: The key to search for.

    Returns:
        The integer value, or None if not found or invalid.
    """
    val = sexpr_get_value(expr, key)
    if val is not None:
        try:
            return int(val)
        except ValueError:
            pass
    return None


def sexpr_has_flag(expr: SExpr, flag: str) -> bool:
    """Check if a flag is present in an S-expression.

    Flags are bare atoms without values, like (pin ... hide).

    Args:
        expr: The S-expression to search.
        flag: The flag name to search for.

    Returns:
        True if the flag is present, False otherwise.
    """
    if not isinstance(expr, list):
        return False

    return flag in expr
