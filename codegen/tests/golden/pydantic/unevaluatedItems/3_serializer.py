"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "string"
  },
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "unevaluatedItems doesn't apply",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Unevaluateditems3Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(list[str], config=ConfigDict(strict=True)): v if not isinstance(v, list) else _adapter.validate_python(v))]

