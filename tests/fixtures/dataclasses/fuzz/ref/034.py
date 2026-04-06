from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaGeneratedSchemaGeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "": {
      "$defs": {
        "": {
          "type": "number"
        }
      }
    }
  },
  "type": "number"
}"""
    root: float = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaGeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "": {
      "$defs": {
        "": {
          "type": "number"
        }
      }
    }
  }
}"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "": {
      "$defs": {
        "": {
          "type": "number"
        }
      }
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "#/$defs//$defs/"
    }
  ]
}"""
    root: typing.Any = dc.jsoncompat_root_field()

GeneratedSchemaGeneratedSchemaGeneratedSchema.__jsoncompat_root_annotation__ = float

GeneratedSchemaGeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
