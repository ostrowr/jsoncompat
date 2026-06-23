from __future__ import annotations

import collections.abc
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
    length: dc.Omittable[str] = dc.field("length", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Item(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

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
    field___proto__: dc.Omittable[float] = dc.field("__proto__", omittable=True)
    constructor: dc.Omittable[float] = dc.field("constructor", omittable=True)
    toString: dc.Omittable[(typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2Branch2 | collections.abc.Sequence[GeneratedSchemaBranch2Item] | float | str | None] = dc.field("toString", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

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
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[GeneratedSchemaItem] | float | str | None) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (
        GeneratedSchemaBranch2Branch2,
        "object",
        (
            ("length", "length", str, True),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaBranch2Item, "root", typing.Any),
    (
        GeneratedSchemaBranch2,
        "object",
        (
            ("__proto__", "field___proto__", float, True),
            ("constructor", "constructor", float, True),
            ("toString", "toString", ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2Branch2 | collections.abc.Sequence[GeneratedSchemaBranch2Item] | float | str | None), True),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaItem, "root", typing.Any),
    (GeneratedSchema, "root", ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[GeneratedSchemaItem] | float | str | None)),
))
