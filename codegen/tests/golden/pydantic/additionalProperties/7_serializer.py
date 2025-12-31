from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict
from pydantic_core import core_schema

_VALIDATE_FORMATS = False

class Additionalproperties7Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "number"
  },
  "propertyNames": {
    "maxLength": 5
  }
}
"""

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")

