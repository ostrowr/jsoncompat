from typing import Annotated, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

_VALIDATE_FORMATS = False

class Ref3Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "percent%field": {
      "type": "integer"
    },
    "slash/field": {
      "type": "integer"
    },
    "tilde~field": {
      "type": "integer"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "percent": {
      "$ref": "#/$defs/percent%25field"
    },
    "slash": {
      "$ref": "#/$defs/slash~1field"
    },
    "tilde": {
      "$ref": "#/$defs/tilde~0field"
    }
  }
}
"""

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    percent: Annotated[int | None, Field(default=None)]
    slash: Annotated[int | None, Field(default=None)]
    tilde: Annotated[int | None, Field(default=None)]

