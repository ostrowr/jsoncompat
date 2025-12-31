"""
Schema:
{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    }
  },
  "$id": "https://test.json-schema.org/ref-dynamicAnchor-same-schema/root",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$ref": "#items"
  },
  "type": "array"
}

Tests:
[
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "An array of strings is valid",
    "valid": true
  },
  {
    "data": [
      "foo",
      42
    ],
    "description": "An array containing non-strings is invalid",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    }
  },
  "$id": "https://test.json-schema.org/ref-dynamicAnchor-same-schema/root",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$ref": "#items"
  },
  "type": "array"
}
"""

_VALIDATE_FORMATS = False

class Dynamicref2Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: list[Any]

