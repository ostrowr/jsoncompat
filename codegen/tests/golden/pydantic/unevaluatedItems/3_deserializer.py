"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "string"
  },
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "unevaluatedItems doesn't apply",
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
    "type": "string"
  },
  "unevaluatedItems": false
}
"""

_VALIDATE_FORMATS = False

class Unevaluateditems3Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

