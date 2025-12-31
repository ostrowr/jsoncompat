from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Dynamicref19Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "bar": {
      "$defs": {
        "content": {
          "$dynamicAnchor": "content",
          "type": "string"
        },
        "item": {
          "$defs": {
            "defaultContent": {
              "$dynamicAnchor": "content",
              "type": "integer"
            }
          },
          "$id": "item",
          "properties": {
            "content": {
              "$dynamicRef": "#content"
            }
          },
          "type": "object"
        }
      },
      "$id": "bar",
      "items": {
        "$ref": "item"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-ref-skips-intermediate-resource/main",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar-item": {
      "$ref": "item"
    }
  },
  "type": "object"
}
"""
    model_config = ConfigDict(extra="allow")
    bar_item: Annotated[Any | None, Field(alias="bar-item", default=None)]

