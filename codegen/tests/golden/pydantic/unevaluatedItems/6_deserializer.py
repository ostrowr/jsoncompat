"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "number"
  },
  "unevaluatedItems": {
    "type": "string"
  }
}

Tests:
[
  {
    "comment": "no elements are considered by unevaluatedItems",
    "data": [
      5,
      6,
      7,
      8
    ],
    "description": "valid under items",
    "valid": true
  },
  {
    "data": [
      "foo",
      "bar",
      "baz"
    ],
    "description": "invalid under items",
    "valid": false
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
    "type": "number"
  },
  "unevaluatedItems": {
    "type": "string"
  }
}
"""

_VALIDATE_FORMATS = False

class Unevaluateditems6Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

