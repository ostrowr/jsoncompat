from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "required": [
        "foo"
      ]
    },
    {
      "properties": {
        "baz": {
          "type": "null"
        }
      },
      "required": [
        "baz"
      ]
    }
  ],
  "properties": {
    "bar": {
      "type": "integer"
    }
  },
  "required": [
    "bar"
  ]
}"""
    bar: int = dc.field("bar")
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
