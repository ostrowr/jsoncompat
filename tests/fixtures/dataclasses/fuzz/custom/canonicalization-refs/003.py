from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "allOf": [
    {
      "$ref": "#/x/allOf/0/properties/value"
    }
  ],
  "x": {
    "allOf": [
      {
        "maxProperties": 1,
        "minProperties": 2,
        "properties": {
          "value": {
            "type": "string"
          }
        },
        "type": "object"
      }
    ]
  }
}"""
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
