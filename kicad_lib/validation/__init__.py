"""KiCad validation.

Contains ERC rules, connection graph analysis, and validation runners.
"""

from .erc import (
    ErcViolation,
    ErcResult,
    ValidationRule,
    WireDanglingRule,
    LabelDanglingRule,
    DuplicateReferenceRule,
    EndpointOffGridRule,
    ErcRunner,
    run_erc,
)

__all__ = [
    "ErcViolation",
    "ErcResult",
    "ValidationRule",
    "WireDanglingRule",
    "LabelDanglingRule",
    "DuplicateReferenceRule",
    "EndpointOffGridRule",
    "ErcRunner",
    "run_erc",
]
