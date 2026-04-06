from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "multipleOf": 1.5,
  "type": "integer"
}"""
    root: int = dc.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = int

JSONCOMPAT_MODEL = GeneratedSchema
