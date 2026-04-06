from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Item(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Item2(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[int]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "multipleOf": 1,
    "type": "integer"
  },
  "minProperties": 0,
  "patternProperties": {
    "f.o": {
      "anyOf": [
        {
          "enum": [
            null
          ]
        },
        {
          "enum": [
            false,
            true
          ]
        },
        {
          "minProperties": 0,
          "properties": {},
          "type": "object"
        },
        {
          "items": true,
          "minItems": 2,
          "type": "array"
        },
        {
          "minLength": 0,
          "type": "string"
        },
        {
          "type": "number"
        }
      ]
    }
  },
  "properties": {
    "bar": {
      "items": true,
      "minItems": 0,
      "type": "array"
    },
    "foo": {
      "items": true,
      "maxItems": 3,
      "minItems": 0,
      "type": "array"
    }
  },
  "type": "object"
}"""
    bar: jsoncompat_dataclasses.Omittable[list[GeneratedSchemaBranch2Item]] = jsoncompat_dataclasses.jsoncompat_field("bar", omittable=True)
    foo: jsoncompat_dataclasses.Omittable[list[GeneratedSchemaBranch2Item2]] = jsoncompat_dataclasses.jsoncompat_field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, int] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "integer"
  },
  "patternProperties": {
    "f.o": {
      "minItems": 2
    }
  },
  "properties": {
    "bar": {
      "type": "array"
    },
    "foo": {
      "maxItems": 3,
      "type": "array"
    }
  }
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch2Item.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2Item2.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("bar", "bar", (list[GeneratedSchemaBranch2Item] | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo", "foo", (list[GeneratedSchemaBranch2Item2] | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, int],
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None)

JSONCOMPAT_MODEL = GeneratedSchema
