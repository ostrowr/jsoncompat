from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluateditems23Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "contains": {
      "const": "a"
    }
  },
  "then": {
    "if": {
      "contains": {
        "const": "b"
      }
    },
    "then": {
      "if": {
        "contains": {
          "const": "c"
        }
      }
    }
  },
  "unevaluatedItems": false
}
"""
    root: Any

