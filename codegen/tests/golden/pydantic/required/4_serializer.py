"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "required": [
    "__proto__",
    "toString",
    "constructor"
  ]
}

Tests:
[
  {
    "data": [],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
  },
  {
    "data": {},
    "description": "none of the properties mentioned",
    "valid": false
  },
  {
    "data": {
      "__proto__": "foo"
    },
    "description": "__proto__ present",
    "valid": false
  },
  {
    "data": {
      "toString": {
        "length": 37
      }
    },
    "description": "toString present",
    "valid": false
  },
  {
    "data": {
      "constructor": {
        "length": 37
      }
    },
    "description": "constructor present",
    "valid": false
  },
  {
    "data": {
      "__proto__": 12,
      "constructor": 37,
      "toString": {
        "length": "foo"
      }
    },
    "description": "all present",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import core_schema

class Required4Serializer(SerializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    proto: Annotated[Any, Field(alias="__proto__")]
    constructor: Any
    to_string: Annotated[Any, Field(alias="toString")]

