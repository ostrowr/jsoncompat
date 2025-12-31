"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "null"
  }
}

Tests:
[
  {
    "data": [
      null
    ],
    "description": "allows null elements",
    "valid": true
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
  "items": {
    "type": "null"
  }
}
"""

_VALIDATE_FORMATS = False

class Items9Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

