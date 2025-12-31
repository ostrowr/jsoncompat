from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Dynamicref12Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
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
"""
    root: Any

