"""
Schema:
{
  "$id": "https://schema/using/format-assertion/false",
  "$schema": "http://localhost:1234/draft2020-12/format-assertion-false.json",
  "format": "ipv4"
}

Tests:
[
  {
    "data": "127.0.0.1",
    "description": "format-assertion: false: valid string",
    "valid": true
  },
  {
    "data": "not-an-ipv4",
    "description": "format-assertion: false: invalid string",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$id": "https://schema/using/format-assertion/false",
  "$schema": "http://localhost:1234/draft2020-12/format-assertion-false.json",
  "format": "ipv4"
}
"""

_VALIDATE_FORMATS = True

class Formatassertion0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

