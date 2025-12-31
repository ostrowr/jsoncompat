"""
Schema:
{
  "$comment": "$id must be evaluated before $ref to get the proper $ref destination",
  "$defs": {
    "bigint": {
      "$anchor": "bigint",
      "$comment": "canonical uri: /ref-and-id2/base.json#/$defs/bigint; another valid uri for this location: /ref-and-id2/base.json#bigint",
      "maximum": 10
    },
    "smallint": {
      "$anchor": "bigint",
      "$comment": "canonical uri: https://example.com/ref-and-id2#/$defs/smallint; another valid uri for this location: https://example.com/ref-and-id2/#bigint",
      "$id": "https://example.com/draft2020-12/ref-and-id2/",
      "maximum": 2
    }
  },
  "$id": "https://example.com/draft2020-12/ref-and-id2/base.json",
  "$ref": "#bigint",
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

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ref19Deserializer(DeserializerRootModel):
    root: Annotated[float, Field(le=10.0)]

