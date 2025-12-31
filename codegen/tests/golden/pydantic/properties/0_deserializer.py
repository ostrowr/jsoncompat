"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "type": "string"
    },
    "foo": {
      "type": "integer"
    }
  }
}

Tests:
[
  {
    "data": {
      "bar": "baz",
      "foo": 1
    },
    "description": "both properties present and valid is valid",
    "valid": true
  },
  {
    "data": {
      "bar": {},
      "foo": 1
    },
    "description": "one property invalid is invalid",
    "valid": false
  },
  {
    "data": {
      "bar": {},
      "foo": []
    },
    "description": "both properties invalid is invalid",
    "valid": false
  },
  {
    "data": {
      "quux": []
    },
    "description": "doesn't invalidate other properties",
    "valid": true
  },
  {
    "data": [],
    "description": "ignores arrays",
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

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import core_schema

class Properties0Deserializer(DeserializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    bar: Annotated[str | None, Field(default=None)]
    foo: Annotated[int | None, Field(default=None)]

