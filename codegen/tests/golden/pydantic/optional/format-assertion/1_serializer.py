from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = True

class Formatassertion1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$id": "https://schema/using/format-assertion/true",
  "$schema": "http://localhost:1234/draft2020-12/format-assertion-true.json",
  "format": "ipv4"
}
"""
    root: Any

