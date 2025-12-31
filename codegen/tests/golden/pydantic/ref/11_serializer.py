"""
Schema:
{
  "$defs": {
    "node": {
      "$id": "http://localhost:1234/draft2020-12/node",
      "description": "node",
      "properties": {
        "subtree": {
          "$ref": "tree"
        },
        "value": {
          "type": "number"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/tree",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "tree of nodes",
  "properties": {
    "meta": {
      "type": "string"
    },
    "nodes": {
      "items": {
        "$ref": "node"
      },
      "type": "array"
    }
  },
  "required": [
    "meta",
    "nodes"
  ],
  "type": "object"
}

Tests:
[
  {
    "data": {
      "meta": "root",
      "nodes": [
        {
          "subtree": {
            "meta": "child",
            "nodes": [
              {
                "value": 1.1
              },
              {
                "value": 1.2
              }
            ]
          },
          "value": 1
        },
        {
          "subtree": {
            "meta": "child",
            "nodes": [
              {
                "value": 2.1
              },
              {
                "value": 2.2
              }
            ]
          },
          "value": 2
        }
      ]
    },
    "description": "valid tree",
    "valid": true
  },
  {
    "data": {
      "meta": "root",
      "nodes": [
        {
          "subtree": {
            "meta": "child",
            "nodes": [
              {
                "value": "string is invalid"
              },
              {
                "value": 1.2
              }
            ]
          },
          "value": 1
        },
        {
          "subtree": {
            "meta": "child",
            "nodes": [
              {
                "value": 2.1
              },
              {
                "value": 2.2
              }
            ]
          },
          "value": 2
        }
      ]
    },
    "description": "invalid tree",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class ModelSerializer(SerializerBase):
    """node"""
    model_config = ConfigDict(extra="allow")
    subtree: Annotated[Any | None, Field(default=None)]
    value: float

class Ref11Serializer(SerializerBase):
    """tree of nodes"""
    model_config = ConfigDict(extra="allow")
    meta: str
    nodes: list[ModelSerializer]

