"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedItems": true
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
    "description": "with unevaluated items",
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
  "unevaluatedItems": true
}
"""

_VALIDATE_FORMATS = False

class Unevaluateditems0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

