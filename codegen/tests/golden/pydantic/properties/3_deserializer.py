"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo\tbar": {
      "type": "number"
    },
    "foo\nbar": {
      "type": "number"
    },
    "foo\fbar": {
      "type": "number"
    },
    "foo\rbar": {
      "type": "number"
    },
    "foo\"bar": {
      "type": "number"
    },
    "foo\\bar": {
      "type": "number"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo\tbar": 1,
      "foo\nbar": 1,
      "foo\fbar": 1,
      "foo\rbar": 1,
      "foo\"bar": 1,
      "foo\\bar": 1
    },
    "description": "object with all numbers is valid",
    "valid": true
  },
  {
    "data": {
      "foo\tbar": "1",
      "foo\nbar": "1",
      "foo\fbar": "1",
      "foo\rbar": "1",
      "foo\"bar": "1",
      "foo\\bar": "1"
    },
    "description": "object with strings is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import core_schema

class Properties3Deserializer(DeserializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo_bar: Annotated[float | None, Field(alias="foo\tbar", default=None)]
    foo_bar_2: Annotated[float | None, Field(alias="foo\nbar", default=None)]
    foo_bar_3: Annotated[float | None, Field(alias="foo\fbar", default=None)]
    foo_bar_4: Annotated[float | None, Field(alias="foo\rbar", default=None)]
    foo_bar_5: Annotated[float | None, Field(alias="foo\"bar", default=None)]
    foo_bar_6: Annotated[float | None, Field(alias="foo\\bar", default=None)]

