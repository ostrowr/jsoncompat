from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"anyOf\":[{\"enum\":[\"a\",\"b\"]},{\"type\":\"null\"}]}"
    root: ((typing.Literal["a"] | typing.Literal["b"]) | None) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal["a"] | typing.Literal["b"]) | None)

JSONCOMPAT_MODEL = GeneratedSchema
