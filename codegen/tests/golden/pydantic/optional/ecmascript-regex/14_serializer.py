"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\p{digit}+$"
}

Tests:
[
  {
    "data": "42",
    "description": "ascii digits",
    "valid": true
  },
  {
    "data": "-%#",
    "description": "ascii non-digits",
    "valid": false
  },
  {
    "data": "৪২",
    "description": "non-ascii digits (BENGALI DIGIT FOUR, BENGALI DIGIT TWO)",
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
  "pattern": "^\\p{digit}+$"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex14Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

