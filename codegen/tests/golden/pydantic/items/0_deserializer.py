"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "integer"
  }
}

Tests:
[
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "valid items",
    "valid": true
  },
  {
    "data": [
      1,
      "x"
    ],
    "description": "wrong type of items",
    "valid": false
  },
  {
    "data": {
      "foo": "bar"
    },
    "description": "ignores non-arrays",
    "valid": true
  },
  {
    "data": {
      "0": "invalid",
      "length": 1
    },
    "description": "JavaScript pseudo-array is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Items0Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(list[int], config=ConfigDict(strict=True)): v if not isinstance(v, list) else _adapter.validate_python(v))]

