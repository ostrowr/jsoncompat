"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": null
}

Tests:
[
  {
    "data": null,
    "description": "null is valid",
    "valid": true
  },
  {
    "data": 0,
    "description": "not null is invalid",
    "valid": false
  }
]
"""

from typing import ClassVar, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": null
}
"""

_VALIDATE_FORMATS = False

class Const3Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Literal[None]

