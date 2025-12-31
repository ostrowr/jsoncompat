"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "not": {
    "$comment": "this subschema must still produce annotations internally, even though the 'not' will ultimately discard them",
    "anyOf": [
      true,
      {
        "properties": {
          "foo": true
        }
      }
    ],
    "unevaluatedProperties": false
  }
}

Tests:
[
  {
    "data": {
      "bar": 1
    },
    "description": "unevaluated property",
    "valid": true
  },
  {
    "data": {
      "foo": 1
    },
    "description": "annotations are still collected inside a 'not'",
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
  "not": {
    "$comment": "this subschema must still produce annotations internally, even though the 'not' will ultimately discard them",
    "anyOf": [
      true,
      {
        "properties": {
          "foo": true
        }
      }
    ],
    "unevaluatedProperties": false
  }
}
"""

_VALIDATE_FORMATS = False

class Not8Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

