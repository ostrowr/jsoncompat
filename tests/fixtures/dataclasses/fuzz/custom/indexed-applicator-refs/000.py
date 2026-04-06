from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTuple(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "tuple": {
      "items": false,
      "prefixItems": [
        {
          "type": "integer"
        },
        {
          "type": "string"
        }
      ]
    }
  },
  "items": false,
  "prefixItems": [
    {
      "type": "integer"
    },
    {
      "type": "string"
    }
  ]
}"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "tuple": {
      "items": false,
      "prefixItems": [
        {
          "type": "integer"
        },
        {
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "#/$defs/tuple/prefixItems/1"
    }
  ]
}"""
    root: typing.Any = dc.jsoncompat_root_field()

GeneratedSchemaTuple.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
