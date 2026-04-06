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
    root: float = dc.root_field()

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
    root: typing.Any = dc.root_field()

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
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
