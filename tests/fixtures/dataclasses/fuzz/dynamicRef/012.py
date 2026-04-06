from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaStart(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$comment": "this is the landing spot from $ref",
  "$defs": {
    "start": {
      "$comment": "this is the landing spot from $ref",
      "$dynamicRef": "inner_scope#thingy",
      "$id": "start"
    },
    "thingy": {
      "$comment": "this is the first stop for the $dynamicRef",
      "$dynamicAnchor": "thingy",
      "$id": "inner_scope",
      "type": "string"
    }
  },
  "$dynamicRef": "inner_scope#thingy",
  "$id": "start"
}"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaThingy(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$comment": "this is the first stop for the $dynamicRef",
  "$defs": {
    "start": {
      "$comment": "this is the landing spot from $ref",
      "$dynamicRef": "inner_scope#thingy",
      "$id": "start"
    },
    "thingy": {
      "$comment": "this is the first stop for the $dynamicRef",
      "$dynamicAnchor": "thingy",
      "$id": "inner_scope",
      "type": "string"
    }
  },
  "$dynamicAnchor": "thingy",
  "$id": "inner_scope",
  "type": "string"
}"""
    root: str = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "start": {
      "$comment": "this is the landing spot from $ref",
      "$dynamicRef": "inner_scope#thingy",
      "$id": "start"
    },
    "thingy": {
      "$comment": "this is the first stop for the $dynamicRef",
      "$dynamicAnchor": "thingy",
      "$id": "inner_scope",
      "type": "string"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-ref-leaving-dynamic-scope/main",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "$defs": {
      "thingy": {
        "$comment": "this is first_scope#thingy",
        "$dynamicAnchor": "thingy",
        "type": "number"
      }
    },
    "$id": "first_scope"
  },
  "then": {
    "$defs": {
      "thingy": {
        "$comment": "this is second_scope#thingy, the final destination of the $dynamicRef",
        "$dynamicAnchor": "thingy",
        "type": "null"
      }
    },
    "$id": "second_scope",
    "$ref": "start"
  }
}"""
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
