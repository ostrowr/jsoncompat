"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMaximum": 9.727837981879871e26
}

Tests:
[
  {
    "data": 9.727837981879871e26,
    "description": "comparison works for high numbers",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMaximum": 9.727837981879871e26
}
"""

_VALIDATE_FORMATS = False

class Bignum4Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

