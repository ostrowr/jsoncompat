"""
Schema:
{
  "$ref": "http://localhost:1234/draft2020-12/detached-ref.json#/$defs/foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "number is valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "non-number is invalid",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$ref": "http://localhost:1234/draft2020-12/detached-ref.json#/$defs/foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Refremote14Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

