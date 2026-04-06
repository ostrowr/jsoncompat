from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "oneOf": [
    {
      "type": "string"
    },
    {
      "type": "boolean"
    }
  ]
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | str) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | str)

JSONCOMPAT_MODEL = GeneratedSchema
