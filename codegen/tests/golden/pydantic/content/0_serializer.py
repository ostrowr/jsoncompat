"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contentMediaType": "application/json"
}

Tests:
[
  {
    "data": "{\"foo\": \"bar\"}",
    "description": "a valid JSON document",
    "valid": true
  },
  {
    "data": "{:}",
    "description": "an invalid JSON document; validates true",
    "valid": true
  },
  {
    "data": 100,
    "description": "ignores non-strings",
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
  "contentMediaType": "application/json"
}
"""

_VALIDATE_FORMATS = False

class Content0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

