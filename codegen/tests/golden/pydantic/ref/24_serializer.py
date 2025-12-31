"""
Schema:
{
  "$comment": "RFC 8141 §2.3.2",
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:example:weather?=op=map&lat=39.56&lon=-104.85&datetime=1969-07-21T02:56:15Z",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/bar"
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

from typing import Annotated, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

_JSON_SCHEMA = r"""
{
  "$comment": "RFC 8141 §2.3.2",
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:example:weather?=op=map&lat=39.56&lon=-104.85&datetime=1969-07-21T02:56:15Z",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/bar"
    }
  }
}
"""

_VALIDATE_FORMATS = False

class Ref24Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]

