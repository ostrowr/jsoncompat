"""
Schema:
{
  "$comment": "$id must be evaluated before $ref to get the proper $ref destination",
  "$defs": {
    "bigint": {
      "$comment": "canonical uri: https://example.com/ref-and-id1/int.json",
      "$id": "int.json",
      "maximum": 10
    },
    "smallint": {
      "$comment": "canonical uri: https://example.com/ref-and-id1-int.json",
      "$id": "/draft2020-12/ref-and-id1-int.json",
      "maximum": 2
    }
  },
  "$id": "https://example.com/draft2020-12/ref-and-id1/base.json",
  "$ref": "int.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 5,
    "description": "data is valid against first definition",
    "valid": true
  },
  {
    "data": 50,
    "description": "data is invalid against first definition",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Ref18Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[float, Field(le=10.0)], config=ConfigDict(strict=True)): v if isinstance(v, bool) or not isinstance(v, (int, float)) else _adapter.validate_python(v))]

