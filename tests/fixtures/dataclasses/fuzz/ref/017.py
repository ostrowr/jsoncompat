from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaX(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "x": {
      "$id": "http://example.com/b/c.json",
      "not": {
        "$defs": {
          "y": {
            "$id": "d.json",
            "type": "number"
          }
        }
      }
    }
  },
  "$id": "http://example.com/b/c.json",
  "not": {
    "$defs": {
      "y": {
        "$id": "d.json",
        "type": "number"
      }
    }
  }
}"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "x": {
      "$id": "http://example.com/b/c.json",
      "not": {
        "$defs": {
          "y": {
            "$id": "d.json",
            "type": "number"
          }
        }
      }
    }
  },
  "$id": "http://example.com/a.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "http://example.com/b/d.json"
    }
  ]
}"""
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
