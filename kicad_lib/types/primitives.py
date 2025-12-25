"""Primitive types for KiCad file representation.

Contains immutable value types for coordinates, styling, and effects
used throughout KiCad schematic and symbol files.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional
import uuid as uuid_module


@dataclass(frozen=True)
class Point:
    """A 2D coordinate point.

    Attributes:
        x: X coordinate in millimeters.
        y: Y coordinate in millimeters.
    """

    x: float
    y: float

    def translate(self, dx: float, dy: float) -> Point:
        """Returns a new Point translated by (dx, dy)."""
        return Point(self.x + dx, self.y + dy)

    def rotate_90(self) -> Point:
        """Returns a new Point rotated 90 degrees clockwise around origin."""
        return Point(-self.y, self.x)

    def rotate_180(self) -> Point:
        """Returns a new Point rotated 180 degrees around origin."""
        return Point(-self.x, -self.y)

    def rotate_270(self) -> Point:
        """Returns a new Point rotated 270 degrees clockwise around origin."""
        return Point(self.y, -self.x)

    def scale(self, factor: float) -> Point:
        """Returns a new Point scaled by factor."""
        return Point(self.x * factor, self.y * factor)

    def distance_to(self, other: Point) -> float:
        """Returns the Euclidean distance to another point."""
        dx = self.x - other.x
        dy = self.y - other.y
        return (dx * dx + dy * dy) ** 0.5

    def to_tuple(self) -> tuple[float, float]:
        """Returns (x, y) tuple."""
        return (self.x, self.y)


@dataclass(frozen=True)
class Position:
    """A 2D position with rotation angle.

    Attributes:
        x: X coordinate in millimeters.
        y: Y coordinate in millimeters.
        angle: Rotation angle in degrees (0-360).
    """

    x: float
    y: float
    angle: float = 0.0

    @property
    def point(self) -> Point:
        """Returns the position as a Point (without rotation)."""
        return Point(self.x, self.y)

    def translate(self, dx: float, dy: float) -> Position:
        """Returns a new Position translated by (dx, dy)."""
        return Position(self.x + dx, self.y + dy, self.angle)

    def rotate(self, degrees: float) -> Position:
        """Returns a new Position with angle increased by degrees."""
        new_angle = (self.angle + degrees) % 360
        return Position(self.x, self.y, new_angle)

    def with_angle(self, angle: float) -> Position:
        """Returns a new Position with the specified angle."""
        return Position(self.x, self.y, angle)


@dataclass(frozen=True)
class UUID:
    """A unique identifier for KiCad objects.

    Attributes:
        value: The UUID string in standard format.
    """

    value: str

    @classmethod
    def generate(cls) -> UUID:
        """Generates a new random UUID."""
        return cls(str(uuid_module.uuid4()))

    @classmethod
    def from_string(cls, s: str) -> UUID:
        """Creates a UUID from a string, validating format."""
        # KiCad uses standard UUID format
        uuid_module.UUID(s)  # Raises ValueError if invalid
        return cls(s)

    def __str__(self) -> str:
        return self.value


@dataclass(frozen=True)
class Color:
    """An RGBA color.

    Attributes:
        r: Red component (0-255).
        g: Green component (0-255).
        b: Blue component (0-255).
        a: Alpha component (0.0-1.0).
    """

    r: int
    g: int
    b: int
    a: float = 1.0

    @classmethod
    def from_hex(cls, hex_str: str) -> Color:
        """Creates a Color from a hex string like '#RRGGBB' or '#RRGGBBAA'."""
        hex_str = hex_str.lstrip("#")
        if len(hex_str) == 6:
            r = int(hex_str[0:2], 16)
            g = int(hex_str[2:4], 16)
            b = int(hex_str[4:6], 16)
            return cls(r, g, b)
        elif len(hex_str) == 8:
            r = int(hex_str[0:2], 16)
            g = int(hex_str[2:4], 16)
            b = int(hex_str[4:6], 16)
            a = int(hex_str[6:8], 16) / 255.0
            return cls(r, g, b, a)
        raise ValueError(f"Invalid hex color: {hex_str}")

    def to_hex(self) -> str:
        """Returns the color as a hex string '#RRGGBB'."""
        return f"#{self.r:02x}{self.g:02x}{self.b:02x}"

    def with_alpha(self, alpha: float) -> Color:
        """Returns a new Color with the specified alpha."""
        return Color(self.r, self.g, self.b, alpha)


# Common colors
COLOR_BLACK = Color(0, 0, 0)
COLOR_WHITE = Color(255, 255, 255)
COLOR_RED = Color(255, 0, 0)
COLOR_GREEN = Color(0, 255, 0)
COLOR_BLUE = Color(0, 0, 255)
COLOR_YELLOW = Color(255, 255, 0)
COLOR_TRANSPARENT = Color(0, 0, 0, 0.0)


@dataclass(frozen=True)
class Stroke:
    """Line stroke styling.

    Attributes:
        width: Line width in millimeters.
        type: Line type (solid, dash, dot, dash_dot, dash_dot_dot, default).
        color: Stroke color (optional, uses default if None).
    """

    width: float
    type: str = "default"
    color: Optional[Color] = None

    def with_width(self, width: float) -> Stroke:
        """Returns a new Stroke with the specified width."""
        return Stroke(width, self.type, self.color)

    def with_type(self, stroke_type: str) -> Stroke:
        """Returns a new Stroke with the specified type."""
        return Stroke(self.width, stroke_type, self.color)

    def with_color(self, color: Color) -> Stroke:
        """Returns a new Stroke with the specified color."""
        return Stroke(self.width, self.type, color)


# Default stroke for schematic wires
STROKE_DEFAULT = Stroke(0.0, "default")


@dataclass(frozen=True)
class Fill:
    """Shape fill styling.

    Attributes:
        type: Fill type (none, outline, background, color).
        color: Fill color (optional, used when type is 'color').
    """

    type: str = "none"
    color: Optional[Color] = None

    @classmethod
    def none(cls) -> Fill:
        """Returns a Fill with no fill."""
        return cls("none")

    @classmethod
    def outline(cls) -> Fill:
        """Returns a Fill that fills with the outline color."""
        return cls("outline")

    @classmethod
    def background(cls) -> Fill:
        """Returns a Fill that uses the background color."""
        return cls("background")

    @classmethod
    def solid(cls, color: Color) -> Fill:
        """Returns a solid Fill with the specified color."""
        return cls("color", color)


FILL_NONE = Fill("none")
FILL_OUTLINE = Fill("outline")
FILL_BACKGROUND = Fill("background")


@dataclass(frozen=True)
class Font:
    """Text font specification.

    Attributes:
        face: Font face name (optional).
        size_x: Horizontal size in millimeters.
        size_y: Vertical size in millimeters.
        thickness: Line thickness for stroke fonts.
        bold: Whether text is bold.
        italic: Whether text is italic.
        line_spacing: Line spacing multiplier.
    """

    face: Optional[str] = None
    size_x: float = 1.27
    size_y: float = 1.27
    thickness: float = 0.254
    bold: bool = False
    italic: bool = False
    line_spacing: float = 1.0

    def with_size(self, size: float) -> Font:
        """Returns a new Font with symmetric size."""
        return Font(
            self.face,
            size,
            size,
            self.thickness,
            self.bold,
            self.italic,
            self.line_spacing,
        )

    def with_bold(self, bold: bool = True) -> Font:
        """Returns a new Font with bold setting."""
        return Font(
            self.face,
            self.size_x,
            self.size_y,
            self.thickness,
            bold,
            self.italic,
            self.line_spacing,
        )

    def with_italic(self, italic: bool = True) -> Font:
        """Returns a new Font with italic setting."""
        return Font(
            self.face,
            self.size_x,
            self.size_y,
            self.thickness,
            self.bold,
            italic,
            self.line_spacing,
        )


FONT_DEFAULT = Font()


@dataclass(frozen=True)
class Effects:
    """Text effects and justification.

    Attributes:
        font: Font specification.
        justify_h: Horizontal justification (left, center, right).
        justify_v: Vertical justification (top, center, bottom).
        mirror: Whether text is mirrored.
        hide: Whether text is hidden.
    """

    font: Font = field(default_factory=lambda: FONT_DEFAULT)
    justify_h: str = "center"
    justify_v: str = "center"
    mirror: bool = False
    hide: bool = False

    def with_font(self, font: Font) -> Effects:
        """Returns a new Effects with the specified font."""
        return Effects(font, self.justify_h, self.justify_v, self.mirror, self.hide)

    def with_justify(
        self, horizontal: str = "center", vertical: str = "center"
    ) -> Effects:
        """Returns a new Effects with the specified justification."""
        return Effects(self.font, horizontal, vertical, self.mirror, self.hide)

    def with_hide(self, hide: bool = True) -> Effects:
        """Returns a new Effects with the hide setting."""
        return Effects(self.font, self.justify_h, self.justify_v, self.mirror, hide)

    def left(self) -> Effects:
        """Returns a new Effects with left horizontal justification."""
        return self.with_justify("left", self.justify_v)

    def right(self) -> Effects:
        """Returns a new Effects with right horizontal justification."""
        return self.with_justify("right", self.justify_v)

    def top(self) -> Effects:
        """Returns a new Effects with top vertical justification."""
        return self.with_justify(self.justify_h, "top")

    def bottom(self) -> Effects:
        """Returns a new Effects with bottom vertical justification."""
        return self.with_justify(self.justify_h, "bottom")


EFFECTS_DEFAULT = Effects()
EFFECTS_HIDDEN = Effects(hide=True)


@dataclass(frozen=True)
class Property:
    """A named property with value and display settings.

    Attributes:
        name: Property name (e.g., 'Reference', 'Value', 'Footprint').
        value: Property value string.
        id: Property ID number.
        position: Display position.
        effects: Text display effects.
    """

    name: str
    value: str
    id: int
    position: Position = field(default_factory=lambda: Position(0, 0))
    effects: Effects = field(default_factory=lambda: EFFECTS_DEFAULT)

    def with_value(self, value: str) -> Property:
        """Returns a new Property with the specified value."""
        return Property(self.name, value, self.id, self.position, self.effects)

    def with_position(self, position: Position) -> Property:
        """Returns a new Property with the specified position."""
        return Property(self.name, self.value, self.id, position, self.effects)

    def with_effects(self, effects: Effects) -> Property:
        """Returns a new Property with the specified effects."""
        return Property(self.name, self.value, self.id, self.position, effects)

    def hidden(self) -> Property:
        """Returns a new Property that is hidden."""
        return self.with_effects(self.effects.with_hide(True))
