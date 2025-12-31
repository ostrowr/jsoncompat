"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "number"
  },
  "unevaluatedItems": {
    "type": "string"
  }
}

Tests:
[
  {
    "comment": "no elements are considered by unevaluatedItems",
    "data": [
      5,
      6,
      7,
      8
    ],
    "description": "valid under items",
    "valid": true
  },
  {
    "data": [
      "foo",
      "bar",
      "baz"
    ],
    "description": "invalid under items",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Unevaluateditems6Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(list[float], config=ConfigDict(strict=True)): v if not isinstance(v, list) else _adapter.validate_python(v))]

