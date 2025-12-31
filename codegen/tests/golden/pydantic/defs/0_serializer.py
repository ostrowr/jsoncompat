"""
Schema:
{
  "$ref": "https://json-schema.org/draft/2020-12/schema",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": {
      "$defs": {
        "foo": {
          "type": "integer"
        }
      }
    },
    "description": "valid definition schema",
    "valid": true
  },
  {
    "data": {
      "$defs": {
        "foo": {
          "type": 1
        }
      }
    },
    "description": "invalid definition schema",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$ref": "https://json-schema.org/draft/2020-12/schema",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Defs0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

