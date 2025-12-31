"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": "hello\u0000there"
}

Tests:
[
  {
    "data": "hello\u0000there",
    "description": "match string with nul",
    "valid": true
  },
  {
    "data": "hellothere",
    "description": "do not match string lacking nul",
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
  "const": "hello\u0000there"
}
"""

_VALIDATE_FORMATS = False

class Const14Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Literal["hello\u0000there"]

