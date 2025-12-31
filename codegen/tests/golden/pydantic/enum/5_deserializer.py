from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict
from pydantic.functional_validators import BeforeValidator

_VALIDATE_FORMATS = False

class Enum5Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    false
  ]
}
"""
    root: Literal[False]

