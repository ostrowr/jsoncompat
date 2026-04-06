from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "integer",
    "string"
  ]
}"""
    root: (int | str) = dc.root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = (int | str)

JSONCOMPAT_MODEL = GeneratedSchema
