from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Item(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Item2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[int]):
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
    bar: dc.Omittable[list[GeneratedSchemaBranch2Item]] = dc.jsoncompat_field("bar", omittable=True)
    foo: dc.Omittable[list[GeneratedSchemaBranch2Item2]] = dc.jsoncompat_field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, int] = dc.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
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
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) = dc.jsoncompat_root_field()

GeneratedSchemaBranch2Item.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2Item2.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("bar", "bar", (list[GeneratedSchemaBranch2Item] | dc.JsoncompatMissingType), omittable=True),
    dc.jsoncompat_field_spec("foo", "foo", (list[GeneratedSchemaBranch2Item2] | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, int],
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None)

JSONCOMPAT_MODEL = GeneratedSchema
