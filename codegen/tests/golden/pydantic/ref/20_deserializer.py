"""
Schema:
{
  "$comment": "URIs do not have to have HTTP(s) schemes",
  "$id": "urn:uuid:deadbeef-1234-ffff-ffff-4321feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minimum": 30,
  "properties": {
    "foo": {
      "$ref": "urn:uuid:deadbeef-1234-ffff-ffff-4321feebdaed"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": 37
    },
    "description": "valid under the URN IDed schema",
    "valid": true
  },
  {
    "data": {
      "foo": 12
    },
    "description": "invalid under the URN IDed schema",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref20Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]

