"""
Schema:
{
  "$defs": {
    "genericList": {
      "$defs": {
        "defaultItemType": {
          "$comment": "Only needed to satisfy bookending requirement",
          "$dynamicAnchor": "itemType"
        }
      },
      "$id": "genericList",
      "properties": {
        "list": {
          "items": {
            "$dynamicRef": "#itemType"
          }
        }
      }
    },
    "numberList": {
      "$defs": {
        "itemType": {
          "$dynamicAnchor": "itemType",
          "type": "number"
        }
      },
      "$id": "numberList",
      "$ref": "genericList"
    },
    "stringList": {
      "$defs": {
        "itemType": {
          "$dynamicAnchor": "itemType",
          "type": "string"
        }
      },
      "$id": "stringList",
      "$ref": "genericList"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-ref-with-multiple-paths/main",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "$ref": "stringList"
  },
  "if": {
    "properties": {
      "kindOfList": {
        "const": "numbers"
      }
    },
    "required": [
      "kindOfList"
    ]
  },
  "then": {
    "$ref": "numberList"
  }
}

Tests:
[
  {
    "data": {
      "kindOfList": "numbers",
      "list": [
        1.1
      ]
    },
    "description": "number list with number values",
    "valid": true
  },
  {
    "data": {
      "kindOfList": "numbers",
      "list": [
        "foo"
      ]
    },
    "description": "number list with string values",
    "valid": false
  },
  {
    "data": {
      "kindOfList": "strings",
      "list": [
        1.1
      ]
    },
    "description": "string list with number values",
    "valid": false
  },
  {
    "data": {
      "kindOfList": "strings",
      "list": [
        "foo"
      ]
    },
    "description": "string list with string values",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dynamicref11Deserializer(DeserializerRootModel):
    root: Any

