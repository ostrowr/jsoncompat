"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {},
    "foo": {}
  },
  "required": [
    "foo"
  ]
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "present required property is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 1
    },
    "description": "non-present required property is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": "",
    "description": "ignores strings",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import core_schema

class Required0Serializer(SerializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    bar: Annotated[Any | None, Field(default=None)]
    foo: Any

