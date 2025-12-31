"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedItems": {
    "type": "string"
  }
}

Tests:
[
  {
    "data": [],
    "description": "with no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo"
    ],
    "description": "with valid unevaluated items",
    "valid": true
  },
  {
    "data": [
      42
    ],
    "description": "with invalid unevaluated items",
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
  "unevaluatedItems": {
    "type": "string"
  }
}
"""

_VALIDATE_FORMATS = False

class Unevaluateditems2Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

