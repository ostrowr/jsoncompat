from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = True

class Formatassertion1Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$id": "https://schema/using/format-assertion/true",
  "$schema": "http://localhost:1234/draft2020-12/format-assertion-true.json",
  "format": "ipv4"
}
"""
    root: Any

