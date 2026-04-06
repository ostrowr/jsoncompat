from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "#/carrier/$defs/name"
    }
  ],
  "carrier": {
    "$anchor": "carrier",
    "$defs": {
      "name": {
        "type": "string"
      }
    },
    "$dynamicAnchor": "dynamic-carrier",
    "$id": "https://example.com/carrier",
    "oneOf": [
      false
    ],
    "title": "carrier metadata",
    "x-jsoncompat": {
      "preserve": true
    }
  }
}"""
    root: typing.Any = dc.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
