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
      "$ref": "#/carrier"
    }
  ],
  "carrier": {
    "allOf": [
      true
    ],
    "title": "true carrier",
    "x-jsoncompat": {
      "note": "keep"
    }
  }
}"""
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
