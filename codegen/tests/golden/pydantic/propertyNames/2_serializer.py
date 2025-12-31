"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": true
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "object with any properties is valid",
    "valid": true
  },
  {
    "data": {},
    "description": "empty object is valid",
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
  "propertyNames": true
}
"""

_VALIDATE_FORMATS = False

class Propertynames2Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

