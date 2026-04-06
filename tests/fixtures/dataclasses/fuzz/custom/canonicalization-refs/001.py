from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "else": {
    "$ref": "#/not"
  },
  "if": true,
  "not": false,
  "then": {
    "type": "string"
  }
}"""
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
