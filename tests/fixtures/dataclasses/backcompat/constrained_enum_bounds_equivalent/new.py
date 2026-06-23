from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "enum": [
    0,
    1
  ],
  "minimum": 1,
  "type": "number"
}"""
    root: (typing.Literal[0] | typing.Literal[1]) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchema, "root", (typing.Literal[0] | typing.Literal[1])),
))
