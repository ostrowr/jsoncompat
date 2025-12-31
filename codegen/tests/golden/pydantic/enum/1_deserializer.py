"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    6,
    "foo",
    [],
    true,
    {
      "foo": 12
    }
  ]
}

Tests:
[
  {
    "data": [],
    "description": "one of the enum is valid",
    "valid": true
  },
  {
    "data": null,
    "description": "something else is invalid",
    "valid": false
  },
  {
    "data": {
      "foo": false
    },
    "description": "objects are deep compared",
    "valid": false
  },
  {
    "data": {
      "foo": 12
    },
    "description": "valid object matches",
    "valid": true
  },
  {
    "data": {
      "boo": 42,
      "foo": 12
    },
    "description": "extra properties in object is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Enum1Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[6, "foo", [], True, {"foo": 12}]: _validate_literal(v, _allowed))]

