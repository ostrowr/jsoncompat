from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "minProperties": 0,
  "properties": {},
  "type": "object"
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "properties": {
        "foo": {
          "prefixItems": [
            true,
            {
              "type": "string"
            }
          ]
        }
      }
    }
  ],
  "properties": {
    "foo": {
      "prefixItems": [
        {
          "type": "string"
        }
      ],
      "unevaluatedItems": false
    }
  }
}"""
    foo: dc.Omittable[(typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[typing.Any] | str | None] = dc.field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
