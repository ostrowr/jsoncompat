"""
Schema:
{
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
}

Tests:
[
  {
    "data": "a string",
    "description": "string matches /$defs/thingy, but the $dynamicRef does not stop here",
    "valid": false
  },
  {
    "data": 42,
    "description": "first_scope is not in dynamic scope for the $dynamicRef",
    "valid": false
  },
  {
    "data": null,
    "description": "/then/$defs/thingy is the final stop for the $dynamicRef",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dynamicref12Deserializer(DeserializerRootModel):
    root: Any

