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
    children: dc.Omittable[list[GeneratedSchema]] = dc.jsoncompat_field("children", omittable=True)
    value: int = dc.jsoncompat_field("value")

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("children", "children", (list[GeneratedSchema] | dc.JsoncompatMissingType), omittable=True),
    dc.jsoncompat_field_spec("value", "value", int),
)

JSONCOMPAT_MODEL = GeneratedSchema
