"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "string"
    }
  ],
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      "foo"
    ],
    "description": "with no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "with unevaluated items",
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
  "prefixItems": [
    {
      "type": "string"
    }
  ],
  "unevaluatedItems": false
}
"""

_VALIDATE_FORMATS = False

class Unevaluateditems4Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

