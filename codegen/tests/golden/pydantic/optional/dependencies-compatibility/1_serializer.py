"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependencies": {
    "bar": []
  }
}

Tests:
[
  {
    "data": {},
    "description": "empty object",
    "valid": true
  },
  {
    "data": {
      "bar": 2
    },
    "description": "object with one property",
    "valid": true
  },
  {
    "data": 1,
    "description": "non-object is valid",
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
  "dependencies": {
    "bar": []
  }
}
"""

_VALIDATE_FORMATS = False

class Dependenciescompatibility1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

