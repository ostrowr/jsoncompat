from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Dynamicref11Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
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
"""
    root: Any

