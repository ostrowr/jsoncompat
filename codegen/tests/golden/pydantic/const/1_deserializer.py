"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": {
    "baz": "bax",
    "foo": "bar"
  }
}

Tests:
[
  {
    "data": {
      "baz": "bax",
      "foo": "bar"
    },
    "description": "same object is valid",
    "valid": true
  },
  {
    "data": {
      "baz": "bax",
      "foo": "bar"
    },
    "description": "same object with different property order is valid",
    "valid": true
  },
  {
    "data": {
      "foo": "bar"
    },
    "description": "another object is invalid",
    "valid": false
  },
  {
    "data": [
      1,
      2
    ],
    "description": "another type is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const1Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[{"baz": "bax", "foo": "bar"}]: _validate_literal(v, _allowed))]

