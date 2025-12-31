"""
Schema:
{
  "$defs": {
    "bar": {
      "$anchor": "something",
      "type": "string"
    }
  },
  "$id": "urn:uuid:deadbeef-1234-ff00-00ff-4321feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "urn:uuid:deadbeef-1234-ff00-00ff-4321feebdaed#something"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": "bar"
    },
    "description": "a string is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 12
    },
    "description": "a non-string is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref26Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]

