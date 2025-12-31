from typing import Annotated, Any, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class ModelSerializer(SerializerBase):
    """node"""
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "__root__": {
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
    },
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
  "$ref": "#/$defs/__root__/properties/nodes/items",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    model_config = ConfigDict(extra="allow")
    subtree: Annotated[Any | None, Field(default=None)]
    value: float

class Ref11Serializer(SerializerBase):
    """tree of nodes"""
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
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
"""
    model_config = ConfigDict(extra="allow")
    meta: str
    nodes: list[ModelSerializer]

