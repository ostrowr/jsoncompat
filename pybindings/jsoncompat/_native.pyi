from typing import Final, Never, final


@final
class JsoncompatMissingType:
    def __new__(cls, _unconstructible: Never, /) -> Never: ...
    def __repr__(self) -> str: ...


JSONCOMPAT_MISSING: Final[JsoncompatMissingType]
