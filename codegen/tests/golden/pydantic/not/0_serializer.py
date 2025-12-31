"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "not": {
    "type": "integer"
  }
}

Tests:
[
  {
    "data": "foo",
    "description": "allowed",
    "valid": true
  },
  {
    "data": 1,
    "description": "disallowed",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "not": {
    "type": "integer"
  }
}
"""

_VALIDATE_FORMATS = False

class Not0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

