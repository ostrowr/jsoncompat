from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaValue(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "Value": {
      "allOf": [
        {
          "$ref": "#/$defs/Value"
        },
        {
          "type": "string"
        }
      ]
    }
  },
  "allOf": [
    {
      "$ref": "#/$defs/Value"
    },
    {
      "type": "string"
    }
  ]
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "Value": {
      "allOf": [
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
  "enum": [
    {
      "value": "leaf"
    }
  ],
  "properties": {
    "value": {
      "$ref": "#/$defs/Value"
    }
  },
  "required": [
    "value"
  ],
  "type": "object"
}"""
    value: GeneratedSchemaValue = dc.field("value")

JSONCOMPAT_MODEL = GeneratedSchema
