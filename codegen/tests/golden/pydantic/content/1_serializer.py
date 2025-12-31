"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contentEncoding": "base64"
}

Tests:
[
  {
    "data": "eyJmb28iOiAiYmFyIn0K",
    "description": "a valid base64 string",
    "valid": true
  },
  {
    "data": "eyJmb28iOi%iYmFyIn0K",
    "description": "an invalid base64 string (% is not a valid character); validates true",
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
  "contentEncoding": "base64"
}
"""

_VALIDATE_FORMATS = False

class Content1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

