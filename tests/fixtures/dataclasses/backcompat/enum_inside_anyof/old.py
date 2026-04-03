from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"enum\":[\"a\",\"b\"]}"
    root: (typing.Literal["a"] | typing.Literal["b"]) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = (typing.Literal["a"] | typing.Literal["b"])

JSONCOMPAT_MODEL = GeneratedSchema
