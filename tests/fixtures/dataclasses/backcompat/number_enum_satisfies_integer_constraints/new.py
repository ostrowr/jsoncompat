from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "enum": [
    4,
    8
  ],
  "maximum": 10,
  "minimum": 0,
  "type": "number"
}"""
    root: (typing.Literal[4] | typing.Literal[8]) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
