from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$id": "http://localhost:1234/draft2020-12/strict-extendible-allof-ref-first.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$defs": {
        "elements": {
          "$dynamicAnchor": "elements",
          "additionalProperties": false,
          "properties": {
            "a": true
          },
          "required": [
            "a"
          ]
        }
      }
    },
    {
      "$ref": "extendible-dynamic-ref.json"
    }
  ]
}"""
    root: typing.Any = dc.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
