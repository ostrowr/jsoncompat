from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN0(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "items": {
        "$ref": "#/$defs/n1"
      },
      "minItems": 0,
      "type": "array"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$ref": "#/$defs/n1"
  },
  "minItems": 0,
  "type": "array"
}"""
    root: collections.abc.Sequence[GeneratedSchemaN1] = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "items": {
        "$ref": "#/$defs/n1"
      },
      "minItems": 0,
      "type": "array"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n2"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "items": {
        "$ref": "#/$defs/n1"
      },
      "minItems": 0,
      "type": "array"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "minLength": 0,
      "type": "string"
    }
  },
  "minLength": 0,
  "type": "string"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    }
  },
  "$id": "https://test.json-schema.org/ref-dynamicAnchor-same-schema/root",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$ref": "#items"
  },
  "type": "array"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
