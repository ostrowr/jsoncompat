from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaValue(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "Value": {
      "anyOf": [
        {
          "$ref": "#/$defs/Value"
        },
        {
          "type": "string"
        }
      ]
    }
  },
  "anyOf": [
    {
      "$ref": "#/$defs/Value"
    },
    {
      "type": "string"
    }
  ]
}"""
    root: (GeneratedSchemaValue | str) = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "Value": {
      "anyOf": [
        {
          "$ref": "#/$defs/Value"
        },
        {
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "value": {
      "$ref": "#/$defs/Value"
    }
  },
  "type": "object"
}"""
    value: dc.Omittable[GeneratedSchemaValue] = dc.field("value", omittable=True)

JSONCOMPAT_MODEL = GeneratedSchema
