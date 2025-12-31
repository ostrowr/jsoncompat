"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "integer"
  }
}

Tests:
[
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "valid items",
    "valid": true
  },
  {
    "data": [
      1,
      "x"
    ],
    "description": "wrong type of items",
    "valid": false
  },
  {
    "data": {
      "foo": "bar"
    },
    "description": "ignores non-arrays",
    "valid": true
  },
  {
    "data": {
      "0": "invalid",
      "length": 1
    },
    "description": "JavaScript pseudo-array is valid",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "integer"
  }
}
"""

_VALIDATE_FORMATS = False

class Items0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

