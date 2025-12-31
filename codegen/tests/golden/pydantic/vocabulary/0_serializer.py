from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

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
    model_config = ConfigDict(extra="allow")
    bad_property: Annotated[Impossible | None, Field(alias="badProperty", default=None)]
    number_property: Annotated[Any | None, Field(alias="numberProperty", default=None)]

