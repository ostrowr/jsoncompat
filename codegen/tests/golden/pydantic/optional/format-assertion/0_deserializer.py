from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = True

class Formatassertion0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$id": "https://schema/using/format-assertion/false",
  "$schema": "http://localhost:1234/draft2020-12/format-assertion-false.json",
  "format": "ipv4"
}
"""
    root: Any

