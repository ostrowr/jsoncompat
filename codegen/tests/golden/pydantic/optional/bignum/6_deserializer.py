from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, TypeAdapter
from pydantic.functional_validators import BeforeValidator

_VALIDATE_FORMATS = False

class Bignum6Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMinimum": -9.727837981879871e26
}
"""
    root: Any

