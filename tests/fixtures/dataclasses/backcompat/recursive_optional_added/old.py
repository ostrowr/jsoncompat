from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "children": {
      "items": {
        "$ref": "#"
      },
      "type": "array"
    },
    "value": {
      "type": "integer"
    }
  },
  "required": [
    "value"
  ],
  "type": "object"
}"""
    children: dc.Omittable[list[GeneratedSchema]] = dc.field("children", omittable=True)
    value: int = dc.field("value")

JSONCOMPAT_MODEL = GeneratedSchema
