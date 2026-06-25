from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "item": {
      "items": false,
      "prefixItems": [
        {
          "$ref": "#/$defs/sub-item"
        },
        {
          "$ref": "#/$defs/sub-item"
        }
      ],
      "type": "array"
    },
    "sub-item": {
      "required": [
        "foo"
      ],
      "type": "object"
    }
  },
  "items": false,
  "prefixItems": [
    {
      "$ref": "#/$defs/sub-item"
    },
    {
      "$ref": "#/$defs/sub-item"
    }
  ],
  "type": "array"
}"""
    root: collections.abc.Sequence[GeneratedSchemaSubItem] = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaSubItemFoo(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaSubItem(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "item": {
      "items": false,
      "prefixItems": [
        {
          "$ref": "#/$defs/sub-item"
        },
        {
          "$ref": "#/$defs/sub-item"
        }
      ],
      "type": "array"
    },
    "sub-item": {
      "required": [
        "foo"
      ],
      "type": "object"
    }
  },
  "required": [
    "foo"
  ],
  "type": "object"
}"""
    foo: GeneratedSchemaSubItemFoo = dc.field("foo")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "item": {
      "items": false,
      "prefixItems": [
        {
          "$ref": "#/$defs/sub-item"
        },
        {
          "$ref": "#/$defs/sub-item"
        }
      ],
      "type": "array"
    },
    "sub-item": {
      "required": [
        "foo"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": false,
  "prefixItems": [
    {
      "$ref": "#/$defs/item"
    },
    {
      "$ref": "#/$defs/item"
    },
    {
      "$ref": "#/$defs/item"
    }
  ],
  "type": "array"
}"""
    root: collections.abc.Sequence[GeneratedSchemaItem] = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
