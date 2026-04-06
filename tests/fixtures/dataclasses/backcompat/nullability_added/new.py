from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "type": [
    "string",
    "null"
  ]
}"""
    root: (str | None) = dc.root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = (str | None)

JSONCOMPAT_MODEL = GeneratedSchema
