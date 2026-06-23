from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "enum": [
    9007199254740993,
    9007199254740995
  ],
  "minimum": 0,
  "type": "integer"
}"""
    root: (typing.Literal[9007199254740993] | typing.Literal[9007199254740995]) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
