"""
Schema:
{
  "$defs": {
    "bar": {
      "$id": "bar",
      "properties": {
        "baz": {
          "$dynamicRef": "extended#meta"
        }
      },
      "type": "object"
    },
    "extended": {
      "$anchor": "meta",
      "$id": "extended",
      "properties": {
        "bar": {
          "$ref": "bar"
        }
      },
      "type": "object"
    }
  },
  "$dynamicAnchor": "meta",
  "$id": "https://test.json-schema.org/relative-dynamic-reference-without-bookend/root",
  "$ref": "extended",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "const": "pass"
    }
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "bar": {
        "baz": {
          "foo": "fail"
        }
      },
      "foo": "pass"
    },
    "description": "The recursive part doesn't need to validate against the root",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any, Literal

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class ModelDeserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    baz: Annotated[Any | None, Field(default=None)]

class Dynamicref10Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[ModelDeserializer | None, Field(default=None)]
    foo: Annotated[Literal["pass"] | None, Field(default=None)]

