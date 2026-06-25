from typing import Final, Self, final


@final
class JsoncompatMissingType:
    def __new__(cls) -> Self: ...
    def __repr__(self) -> str: ...


JSONCOMPAT_MISSING: Final[JsoncompatMissingType]
