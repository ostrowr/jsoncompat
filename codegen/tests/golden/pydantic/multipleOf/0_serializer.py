"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 2
}

Tests:
[
  {
    "data": 10,
    "description": "int by int",
    "valid": true
  },
  {
    "data": 7,
    "description": "int by int fail",
    "valid": false
  },
  {
    "data": "foo",
    "description": "ignores non-numbers",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Multipleof0Serializer(SerializerRootModel):
    root: Annotated[float, Field(multiple_of=2.0)]

