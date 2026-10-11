"""The omission sentinel shared by Python models and their native runtime."""

import sys
from enum import Enum
from typing import Final, NoReturn, final

if sys.version_info >= (3, 13):
    from typing import TypeIs
else:
    from typing_extensions import TypeIs


@final
class MissingType(Enum):
    """The single value representing an omitted field, distinct from JSON null."""

    MISSING = "MISSING"

    def __repr__(self) -> str:
        return "MISSING"

    def __str__(self) -> str:
        return "MISSING"

    def __bool__(self) -> NoReturn:
        raise TypeError("MISSING has no truth value; use 'is MISSING' or 'is not MISSING'")


# Preserve the enum literal type so identity checks narrow both branches.
MISSING: Final = MissingType.MISSING


def is_missing(value: object) -> TypeIs[MissingType]:
    """Test for omission while preserving null and all other present values."""
    return value is MissingType.MISSING
