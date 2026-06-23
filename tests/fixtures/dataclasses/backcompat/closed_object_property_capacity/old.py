from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaX(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "maxProperties": 1,
  "properties": {
    "x": true
  },
  "type": "object"
}"""
    x: dc.Omittable[GeneratedSchemaX] = dc.field("x", omittable=True)

JSONCOMPAT_MODEL = GeneratedSchema
