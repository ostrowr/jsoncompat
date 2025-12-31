from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator
from pydantic_core import core_schema

_VALIDATE_FORMATS = False

class Vocabulary0Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$id": "https://schema/using/no/validation",
  "$schema": "http://localhost:1234/draft2020-12/metaschema-no-validation.json",
  "properties": {
    "badProperty": false,
    "numberProperty": {
      "minimum": 10
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
    bad_property: Annotated[Impossible | None, Field(alias="badProperty", default=None)]
    number_property: Annotated[Any | None, Field(alias="numberProperty", default=None)]

