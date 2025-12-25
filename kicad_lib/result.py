"""Result type for explicit error handling.

Provides Ok and Err variants for representing success and failure states,
with monadic operations for chaining computations.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TypeVar, Generic, Callable, Union, NoReturn

T = TypeVar("T")
U = TypeVar("U")
E = TypeVar("E")
F = TypeVar("F")


@dataclass(frozen=True)
class Ok(Generic[T]):
    """Represents a successful result containing a value."""

    value: T

    def is_ok(self) -> bool:
        """Returns True if this is an Ok result."""
        return True

    def is_err(self) -> bool:
        """Returns False since this is an Ok result."""
        return False

    def unwrap(self) -> T:
        """Returns the contained value."""
        return self.value

    def unwrap_or(self, default: T) -> T:
        """Returns the contained value (ignores default)."""
        return self.value

    def unwrap_err(self) -> NoReturn:
        """Raises ValueError since this is an Ok result."""
        raise ValueError(f"Called unwrap_err on Ok: {self.value}")

    def map(self, fn: Callable[[T], U]) -> Ok[U]:
        """Applies fn to the contained value."""
        return Ok(fn(self.value))

    def map_err(self, fn: Callable[[E], F]) -> Ok[T]:
        """Returns self unchanged since this is Ok."""
        return self

    def and_then(self, fn: Callable[[T], Result[U, E]]) -> Result[U, E]:
        """Applies fn to the contained value, returning its Result."""
        return fn(self.value)

    def or_else(self, fn: Callable[[E], Result[T, F]]) -> Ok[T]:
        """Returns self unchanged since this is Ok."""
        return self


@dataclass(frozen=True)
class Err(Generic[E]):
    """Represents a failed result containing an error."""

    error: E

    def is_ok(self) -> bool:
        """Returns False since this is an Err result."""
        return False

    def is_err(self) -> bool:
        """Returns True if this is an Err result."""
        return True

    def unwrap(self) -> NoReturn:
        """Raises ValueError with the error."""
        raise ValueError(f"Called unwrap on Err: {self.error}")

    def unwrap_or(self, default: T) -> T:
        """Returns the default value."""
        return default

    def unwrap_err(self) -> E:
        """Returns the contained error."""
        return self.error

    def map(self, fn: Callable[[T], U]) -> Err[E]:
        """Returns self unchanged since this is Err."""
        return self

    def map_err(self, fn: Callable[[E], F]) -> Err[F]:
        """Applies fn to the contained error."""
        return Err(fn(self.error))

    def and_then(self, fn: Callable[[T], Result[U, E]]) -> Err[E]:
        """Returns self unchanged since this is Err."""
        return self

    def or_else(self, fn: Callable[[E], Result[T, F]]) -> Result[T, F]:
        """Applies fn to the contained error, returning its Result."""
        return fn(self.error)


Result = Union[Ok[T], Err[E]]
"""Type alias for a result that is either Ok[T] or Err[E]."""


def try_wrap(fn: Callable[[], T], error_type: type[E] = Exception) -> Result[T, E]:
    """Wraps a function call in a Result, catching exceptions of error_type.

    Args:
        fn: A callable that may raise an exception.
        error_type: The exception type to catch (default: Exception).

    Returns:
        Ok(value) if fn() succeeds, Err(exception) if it raises error_type.
    """
    try:
        return Ok(fn())
    except error_type as e:
        return Err(e)
