"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedProperties": {
    "type": "null"
  }
}

Tests:
[
  {
    "data": {
      "foo": null
    },
    "description": "allows null valued properties",
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
  "unevaluatedProperties": {
    "type": "null"
  }
}
"""

_VALIDATE_FORMATS = False

class Unevaluatedproperties36Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

