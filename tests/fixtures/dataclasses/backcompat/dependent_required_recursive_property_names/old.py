from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaName(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "Name": {
      "allOf": [
        {
          "$ref": "#/$defs/Name"
        },
        {
          "type": "string"
        }
      ]
    }
  },
  "allOf": [
    {
      "$ref": "#/$defs/Name"
    },
    {
      "type": "string"
    }
  ]
}"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "Name": {
      "allOf": [
        {
          "$ref": "#/$defs/Name"
        },
        {
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentRequired": {
    "x": [
      "y"
    ]
  },
  "propertyNames": {
    "$ref": "#/$defs/Name"
  },
  "type": "object"
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
