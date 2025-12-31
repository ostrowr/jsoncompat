"""
Schema:
{
  "$defs": {
    "foo": {
      "$defs": {
        "bar": {
          "type": "string"
        }
      },
      "$id": "urn:uuid:deadbeef-4321-ffff-ffff-1234feebdaed",
      "$ref": "#/$defs/bar"
    }
  },
  "$ref": "urn:uuid:deadbeef-4321-ffff-ffff-1234feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": "bar",
    "description": "a string is valid",
    "valid": true
  },
  {
    "data": 12,
    "description": "a non-string is invalid",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$defs": {
    "foo": {
      "$defs": {
        "bar": {
          "type": "string"
        }
      },
      "$id": "urn:uuid:deadbeef-4321-ffff-ffff-1234feebdaed",
      "$ref": "#/$defs/bar"
    }
  },
  "$ref": "urn:uuid:deadbeef-4321-ffff-ffff-1234feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Ref27Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

