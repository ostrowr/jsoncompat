from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Branch2(dc.DataclassAdditionalModel[typing.Any]):
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
    length: dc.Omittable[str] = dc.jsoncompat_field("length", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Item(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[typing.Any]):
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
    __proto__: dc.Omittable[float] = dc.jsoncompat_field("__proto__", omittable=True)
    constructor: dc.Omittable[float] = dc.jsoncompat_field("constructor", omittable=True)
    toString: dc.Omittable[(typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2Branch2 | float | list[GeneratedSchemaBranch2Item] | str | None] = dc.jsoncompat_field("toString", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
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
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) = dc.jsoncompat_root_field()

GeneratedSchemaBranch2Branch2.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("length", "length", (str | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBranch2Item.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("__proto__", "__proto__", (float | dc.JsoncompatMissingType), omittable=True),
    dc.jsoncompat_field_spec("constructor", "constructor", (float | dc.JsoncompatMissingType), omittable=True),
    dc.jsoncompat_field_spec("toString", "toString", (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2Branch2 | float | list[GeneratedSchemaBranch2Item] | str | None) | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None)

JSONCOMPAT_MODEL = GeneratedSchema
