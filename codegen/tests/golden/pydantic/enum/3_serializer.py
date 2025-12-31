"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "enum": [
        "bar"
      ]
    },
    "foo": {
      "enum": [
        "foo"
      ]
    }
  },
  "required": [
    "bar"
  ],
  "type": "object"
}

Tests:
[
  {
    "data": {
      "bar": "bar",
      "foo": "foo"
    },
    "description": "both properties are valid",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "foo": "foot"
    },
    "description": "wrong foo value",
    "valid": false
  },
  {
    "data": {
      "bar": "bart",
      "foo": "foo"
    },
    "description": "wrong bar value",
    "valid": false
  },
  {
    "data": {
      "bar": "bar"
    },
    "description": "missing optional property is valid",
    "valid": true
  },
  {
    "data": {
      "foo": "foo"
    },
    "description": "missing required property is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "missing all properties is invalid",
    "valid": false
  }
]
"""

from typing import Annotated, ClassVar, Literal

from json_schema_codegen_base import DeserializerBase, SerializerBase, _validate_literal
from pydantic import ConfigDict, Field, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "enum": [
        "bar"
      ]
    },
    "foo": {
      "enum": [
        "foo"
      ]
    }
  },
  "required": [
    "bar"
  ],
  "type": "object"
}
"""

_VALIDATE_FORMATS = False

class Enum3Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    model_config = ConfigDict(extra="allow")
    bar: Literal["bar"]
    foo: Annotated[Literal["foo"] | None, Field(default=None)]

