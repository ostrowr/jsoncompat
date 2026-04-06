from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "exclusiveMaximum": 10,
  "exclusiveMinimum": 0,
  "type": "number"
}"""
    root: float = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
