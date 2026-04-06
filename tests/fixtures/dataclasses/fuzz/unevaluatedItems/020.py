from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "minProperties": 0,
  "properties": {},
  "type": "object"
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
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
    foo: jsoncompat_dataclasses.Omittable[(typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[typing.Any] | str | None] = jsoncompat_dataclasses.jsoncompat_field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("foo", "foo", (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[typing.Any] | str | None) | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
