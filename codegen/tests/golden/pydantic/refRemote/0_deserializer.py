"""
Schema:
{
  "$ref": "http://localhost:1234/draft2020-12/integer.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "remote ref valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "remote ref invalid",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$ref": "http://localhost:1234/draft2020-12/integer.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Refremote0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

