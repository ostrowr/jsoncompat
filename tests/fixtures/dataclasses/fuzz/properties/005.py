from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Branch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "minProperties": 0,
  "properties": {
    "length": {
      "minLength": 0,
      "type": "string"
    }
  },
  "type": "object"
}"""
    length: jsoncompat_dataclasses.Omittable[str] = jsoncompat_dataclasses.jsoncompat_field("length", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Item(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "properties": {
    "__proto__": {
      "type": "number"
    },
    "constructor": {
      "type": "number"
    },
    "toString": {
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
          "properties": {
            "length": {
              "minLength": 0,
              "type": "string"
            }
          },
          "type": "object"
        },
        {
          "items": true,
          "minItems": 0,
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
  "type": "object"
}"""
    __proto__: jsoncompat_dataclasses.Omittable[float] = jsoncompat_dataclasses.jsoncompat_field("__proto__", omittable=True)
    constructor: jsoncompat_dataclasses.Omittable[float] = jsoncompat_dataclasses.jsoncompat_field("constructor", omittable=True)
    toString: jsoncompat_dataclasses.Omittable[(typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2Branch2 | float | list[GeneratedSchemaBranch2Item] | str | None] = jsoncompat_dataclasses.jsoncompat_field("toString", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "__proto__": {
      "type": "number"
    },
    "constructor": {
      "type": "number"
    },
    "toString": {
      "properties": {
        "length": {
          "type": "string"
        }
      }
    }
  }
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch2Branch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("length", "length", (str | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBranch2Item.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("__proto__", "__proto__", (float | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("constructor", "constructor", (float | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("toString", "toString", (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2Branch2 | float | list[GeneratedSchemaBranch2Item] | str | None) | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None)

JSONCOMPAT_MODEL = GeneratedSchema
