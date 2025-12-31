"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maxContains": 1
}

Tests:
[
  {
    "data": [
      1
    ],
    "description": "one item valid against lone maxContains",
    "valid": true
  },
  {
    "data": [
      1,
      2
    ],
    "description": "two items still valid against lone maxContains",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maxContains": 1
}
"""

_VALIDATE_FORMATS = False

class Maxcontains0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

