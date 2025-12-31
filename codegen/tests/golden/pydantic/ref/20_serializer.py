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

from typing import Annotated, Any, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

_JSON_SCHEMA = r"""
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
"""

_VALIDATE_FORMATS = False

class Ref20Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]

