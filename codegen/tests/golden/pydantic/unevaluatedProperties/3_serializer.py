"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "type": "string"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {
      "foo": "foo"
    },
    "description": "with no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "foo": "foo"
    },
    "description": "with unevaluated properties",
    "valid": false
  }
]
"""

from typing import Annotated, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "type": "string"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}
"""

_VALIDATE_FORMATS = False

class Unevaluatedproperties3Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]

