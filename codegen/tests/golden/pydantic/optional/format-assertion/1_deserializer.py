"""
Schema:
{
  "$id": "https://schema/using/format-assertion/true",
  "$schema": "http://localhost:1234/draft2020-12/format-assertion-true.json",
  "format": "ipv4"
}

Tests:
[
  {
    "data": "127.0.0.1",
    "description": "format-assertion: true: valid string",
    "valid": true
  },
  {
    "data": "not-an-ipv4",
    "description": "format-assertion: true: invalid string",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$id": "https://schema/using/format-assertion/true",
  "$schema": "http://localhost:1234/draft2020-12/format-assertion-true.json",
  "format": "ipv4"
}
"""

_VALIDATE_FORMATS = True

class Formatassertion1Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

