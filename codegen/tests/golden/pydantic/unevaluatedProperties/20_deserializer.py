"""
Schema:
{
  "$defs": {
    "baseSchema": {
      "$comment": "unevaluatedProperties comes first so it's more likely to catch bugs with implementations that are sensitive to keyword ordering",
      "$defs": {
        "defaultAddons": {
          "$comment": "Needed to satisfy the bookending requirement",
          "$dynamicAnchor": "addons"
        }
      },
      "$dynamicRef": "#addons",
      "$id": "./baseSchema",
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "type": "object",
      "unevaluatedProperties": false
    },
    "derived": {
      "$dynamicAnchor": "addons",
      "properties": {
        "bar": {
          "type": "string"
        }
      }
    }
  },
  "$id": "https://example.com/unevaluated-properties-with-dynamic-ref/derived",
  "$ref": "./baseSchema",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": {
      "bar": "bar",
      "foo": "foo"
    },
    "description": "with no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "baz": "baz",
      "foo": "foo"
    },
    "description": "with unevaluated properties",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Unevaluatedproperties20Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]

