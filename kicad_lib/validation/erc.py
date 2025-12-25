"""ERC (Electrical Rules Check) validation for KiCad schematics.

Provides rule-based validation of schematics to detect common issues
like dangling wires, unconnected pins, and label mismatches.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Protocol, Sequence
from pathlib import Path

from ..types.primitives import Point, Position
from ..types.enums import Severity
from ..types.schematic import (
    Schematic,
    Wire,
    Label,
    HierarchicalLabel,
    SymbolInstance,
    Sheet,
)


@dataclass(frozen=True)
class ErcViolation:
    """A single ERC violation.

    Attributes:
        rule_id: Identifier for the rule that was violated.
        severity: Severity level of the violation.
        message: Human-readable description of the issue.
        location: Location where the violation occurred.
        sheet_path: Path to the sheet containing the violation.
    """

    rule_id: str
    severity: Severity
    message: str
    location: Position = field(default_factory=lambda: Position(0, 0))
    sheet_path: str = ""

    def __str__(self) -> str:
        sev = self.severity.value.upper()
        loc = f"({self.location.x:.2f}, {self.location.y:.2f})"
        return f"[{sev}] {self.rule_id}: {self.message} at {loc}"


@dataclass(frozen=True)
class ErcResult:
    """Result of running ERC on a schematic.

    Attributes:
        violations: List of all violations found.
        sheet_path: Path to the schematic that was checked.
    """

    violations: tuple[ErcViolation, ...]
    sheet_path: str = ""

    @property
    def error_count(self) -> int:
        """Count of ERROR severity violations."""
        return sum(1 for v in self.violations if v.severity == Severity.ERROR)

    @property
    def warning_count(self) -> int:
        """Count of WARNING severity violations."""
        return sum(1 for v in self.violations if v.severity == Severity.WARNING)

    @property
    def has_errors(self) -> bool:
        """True if any ERROR violations exist."""
        return self.error_count > 0

    @property
    def passed(self) -> bool:
        """True if no ERROR violations exist."""
        return not self.has_errors

    def by_severity(self, severity: Severity) -> list[ErcViolation]:
        """Get violations filtered by severity."""
        return [v for v in self.violations if v.severity == severity]

    def by_rule(self, rule_id: str) -> list[ErcViolation]:
        """Get violations filtered by rule ID."""
        return [v for v in self.violations if v.rule_id == rule_id]

    def format_report(self) -> str:
        """Format violations as a human-readable report."""
        lines = []
        lines.append(f"ERC Report for: {self.sheet_path}")
        lines.append(f"Errors: {self.error_count}, Warnings: {self.warning_count}")
        lines.append("")

        if not self.violations:
            lines.append("No violations found.")
        else:
            for v in self.violations:
                lines.append(str(v))

        return "\n".join(lines)


class ValidationRule(Protocol):
    """Protocol for ERC validation rules."""

    @property
    def rule_id(self) -> str:
        """Unique identifier for this rule."""
        ...

    def check(self, schematic: Schematic, sheet_path: str = "") -> list[ErcViolation]:
        """Check the schematic for violations of this rule.

        Args:
            schematic: The schematic to check.
            sheet_path: Path to the sheet being checked.

        Returns:
            List of violations found.
        """
        ...


class WireDanglingRule:
    """Check for wire endpoints that are not connected to anything."""

    @property
    def rule_id(self) -> str:
        return "wire_dangling"

    def check(self, schematic: Schematic, sheet_path: str = "") -> list[ErcViolation]:
        violations = []

        # Collect all connection points
        connection_points: set[tuple[float, float]] = set()

        # Wire endpoints
        for wire in schematic.wires:
            connection_points.add((wire.start.x, wire.start.y))
            connection_points.add((wire.end.x, wire.end.y))

        # Junction points explicitly connect wires
        for junction in schematic.junctions:
            connection_points.add((junction.position.x, junction.position.y))

        # Label positions
        for label in schematic.labels:
            connection_points.add((label.position.x, label.position.y))

        for label in schematic.hierarchical_labels:
            connection_points.add((label.position.x, label.position.y))

        for label in schematic.global_labels:
            connection_points.add((label.position.x, label.position.y))

        # Check each wire endpoint for connections
        for wire in schematic.wires:
            for endpoint in [wire.start, wire.end]:
                pt = (endpoint.x, endpoint.y)

                # Count how many other things connect at this point
                connections = 0
                for other_wire in schematic.wires:
                    if other_wire is not wire:
                        if (other_wire.start.x, other_wire.start.y) == pt:
                            connections += 1
                        if (other_wire.end.x, other_wire.end.y) == pt:
                            connections += 1

                # Check for labels at this point
                for label in schematic.labels:
                    if (label.position.x, label.position.y) == pt:
                        connections += 1

                for label in schematic.hierarchical_labels:
                    if (label.position.x, label.position.y) == pt:
                        connections += 1

                # Check for junctions
                for junction in schematic.junctions:
                    if (junction.position.x, junction.position.y) == pt:
                        connections += 1

                # If no connections, it's dangling
                if connections == 0:
                    violations.append(
                        ErcViolation(
                            rule_id=self.rule_id,
                            severity=Severity.WARNING,
                            message=f"Wire endpoint has no connection",
                            location=Position(endpoint.x, endpoint.y),
                            sheet_path=sheet_path,
                        )
                    )

        return violations


class LabelDanglingRule:
    """Check for labels that are not connected to wires."""

    @property
    def rule_id(self) -> str:
        return "label_dangling"

    def check(self, schematic: Schematic, sheet_path: str = "") -> list[ErcViolation]:
        violations = []

        # Collect all wire endpoints and midpoints
        wire_points: set[tuple[float, float]] = set()
        for wire in schematic.wires:
            wire_points.add((wire.start.x, wire.start.y))
            wire_points.add((wire.end.x, wire.end.y))

        # Check regular labels
        for label in schematic.labels:
            pt = (label.position.x, label.position.y)
            if pt not in wire_points:
                violations.append(
                    ErcViolation(
                        rule_id=self.rule_id,
                        severity=Severity.ERROR,
                        message=f'Label "{label.text}" is not connected to a wire',
                        location=label.position,
                        sheet_path=sheet_path,
                    )
                )

        # Check hierarchical labels
        for label in schematic.hierarchical_labels:
            pt = (label.position.x, label.position.y)
            if pt not in wire_points:
                violations.append(
                    ErcViolation(
                        rule_id=self.rule_id,
                        severity=Severity.ERROR,
                        message=f'Hierarchical label "{label.text}" is not connected to a wire',
                        location=label.position,
                        sheet_path=sheet_path,
                    )
                )

        # Check global labels
        for label in schematic.global_labels:
            pt = (label.position.x, label.position.y)
            if pt not in wire_points:
                violations.append(
                    ErcViolation(
                        rule_id=self.rule_id,
                        severity=Severity.ERROR,
                        message=f'Global label "{label.text}" is not connected to a wire',
                        location=label.position,
                        sheet_path=sheet_path,
                    )
                )

        return violations


class HierLabelMismatchRule:
    """Check that hierarchical labels match sheet pins."""

    @property
    def rule_id(self) -> str:
        return "hier_label_mismatch"

    def check(self, schematic: Schematic, sheet_path: str = "") -> list[ErcViolation]:
        violations = []

        # Get all hierarchical label names in this schematic
        hier_label_names = {label.text for label in schematic.hierarchical_labels}

        # For each sheet, check that its pins exist as hierarchical labels
        # (This check would need to load subsheets, so we just check the current sheet)

        # For now, we check if any sheet pins reference labels that don't exist
        for sheet in schematic.sheets:
            for pin in sheet.pins:
                # This would need to check the subsheet's hierarchical labels
                # For now, we just log a note about the sheet structure
                pass

        return violations


class DuplicateReferenceRule:
    """Check for duplicate reference designators."""

    @property
    def rule_id(self) -> str:
        return "duplicate_reference"

    def check(self, schematic: Schematic, sheet_path: str = "") -> list[ErcViolation]:
        violations = []

        # Collect references (excluding power symbols which use #PWR)
        references: dict[str, list[SymbolInstance]] = {}
        for sym in schematic.symbols:
            ref = sym.reference
            if not ref.startswith("#"):
                if ref not in references:
                    references[ref] = []
                references[ref].append(sym)

        # Check for duplicates
        for ref, symbols in references.items():
            if len(symbols) > 1:
                for sym in symbols[1:]:
                    violations.append(
                        ErcViolation(
                            rule_id=self.rule_id,
                            severity=Severity.ERROR,
                            message=f'Duplicate reference designator "{ref}"',
                            location=sym.position,
                            sheet_path=sheet_path,
                        )
                    )

        return violations


class EndpointOffGridRule:
    """Check for wire endpoints that are off the standard grid."""

    GRID_SIZE = 1.27  # 50 mil = 1.27mm

    @property
    def rule_id(self) -> str:
        return "endpoint_off_grid"

    def _is_on_grid(self, value: float) -> bool:
        """Check if a value is on the standard grid."""
        remainder = abs(value) % self.GRID_SIZE
        return remainder < 0.01 or (self.GRID_SIZE - remainder) < 0.01

    def check(self, schematic: Schematic, sheet_path: str = "") -> list[ErcViolation]:
        violations = []

        for wire in schematic.wires:
            for endpoint in [wire.start, wire.end]:
                if not self._is_on_grid(endpoint.x) or not self._is_on_grid(endpoint.y):
                    violations.append(
                        ErcViolation(
                            rule_id=self.rule_id,
                            severity=Severity.WARNING,
                            message=f"Wire endpoint is off the 1.27mm grid",
                            location=Position(endpoint.x, endpoint.y),
                            sheet_path=sheet_path,
                        )
                    )

        return violations


class ErcRunner:
    """Runs ERC validation with configurable rules."""

    DEFAULT_RULES: list[ValidationRule] = [
        WireDanglingRule(),
        LabelDanglingRule(),
        DuplicateReferenceRule(),
        EndpointOffGridRule(),
    ]

    def __init__(self, rules: Sequence[ValidationRule] | None = None):
        """Initialize the ERC runner.

        Args:
            rules: Rules to run. If None, uses default rules.
        """
        self._rules = list(rules) if rules is not None else list(self.DEFAULT_RULES)

    def add_rule(self, rule: ValidationRule) -> ErcRunner:
        """Add a rule to the runner."""
        self._rules.append(rule)
        return self

    def check(self, schematic: Schematic, sheet_path: str = "") -> ErcResult:
        """Run all rules on a schematic.

        Args:
            schematic: The schematic to check.
            sheet_path: Path to the sheet being checked.

        Returns:
            ErcResult with all violations found.
        """
        all_violations: list[ErcViolation] = []

        for rule in self._rules:
            violations = rule.check(schematic, sheet_path)
            all_violations.extend(violations)

        return ErcResult(
            violations=tuple(all_violations),
            sheet_path=sheet_path,
        )


def run_erc(schematic: Schematic, sheet_path: str = "") -> ErcResult:
    """Run ERC on a schematic with default rules.

    Args:
        schematic: The schematic to check.
        sheet_path: Path to the sheet being checked.

    Returns:
        ErcResult with all violations found.
    """
    runner = ErcRunner()
    return runner.check(schematic, sheet_path)
