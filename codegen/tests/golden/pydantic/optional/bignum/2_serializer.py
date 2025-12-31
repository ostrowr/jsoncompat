"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "string"
}

Tests:
[
  {
    "data": 9.82492837492349e52,
    "description": "a bignum is not a string",
    "valid": false
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Bignum2Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str

