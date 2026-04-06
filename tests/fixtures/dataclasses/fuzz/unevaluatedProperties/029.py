from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFoo(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "properties": {
    "bar": {
      "type": "string"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}"""
    bar: dc.Omittable[str] = dc.jsoncompat_field("bar", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "properties": {
        "foo": {
          "properties": {
            "faz": {
              "type": "string"
            }
          }
        }
      }
    }
  ],
  "properties": {
    "foo": {
      "properties": {
        "bar": {
          "type": "string"
        }
      },
      "type": "object",
      "unevaluatedProperties": false
    }
  },
  "type": "object"
}"""
    foo: dc.Omittable[GeneratedSchemaFoo] = dc.jsoncompat_field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.jsoncompat_extra_field()

GeneratedSchemaFoo.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("bar", "bar", (str | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("foo", "foo", (GeneratedSchemaFoo | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
