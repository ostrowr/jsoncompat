"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "duration"
}

Tests:
[
  {
    "data": 12,
    "description": "all string formats ignore integers",
    "valid": true
  },
  {
    "data": 13.7,
    "description": "all string formats ignore floats",
    "valid": true
  },
  {
    "data": {},
    "description": "all string formats ignore objects",
    "valid": true
  },
  {
    "data": [],
    "description": "all string formats ignore arrays",
    "valid": true
  },
  {
    "data": false,
    "description": "all string formats ignore booleans",
    "valid": true
  },
  {
    "data": null,
    "description": "all string formats ignore nulls",
    "valid": true
  },
  {
    "data": "P4DT12H30M5S",
    "description": "a valid duration string",
    "valid": true
  },
  {
    "data": "PT1D",
    "description": "an invalid duration string",
    "valid": false
  },
  {
    "data": "4DT12H30M5S",
    "description": "must start with P",
    "valid": false
  },
  {
    "data": "P",
    "description": "no elements present",
    "valid": false
  },
  {
    "data": "P1YT",
    "description": "no time elements present",
    "valid": false
  },
  {
    "data": "PT",
    "description": "no date or time elements present",
    "valid": false
  },
  {
    "data": "P2D1Y",
    "description": "elements out of order",
    "valid": false
  },
  {
    "data": "P1D2H",
    "description": "missing time separator",
    "valid": false
  },
  {
    "data": "P2S",
    "description": "time element in the date position",
    "valid": false
  },
  {
    "data": "P4Y",
    "description": "four years duration",
    "valid": true
  },
  {
    "data": "PT0S",
    "description": "zero time, in seconds",
    "valid": true
  },
  {
    "data": "P0D",
    "description": "zero time, in days",
    "valid": true
  },
  {
    "data": "P1M",
    "description": "one month duration",
    "valid": true
  },
  {
    "data": "PT1M",
    "description": "one minute duration",
    "valid": true
  },
  {
    "data": "PT36H",
    "description": "one and a half days, in hours",
    "valid": true
  },
  {
    "data": "P1DT12H",
    "description": "one and a half days, in days and hours",
    "valid": true
  },
  {
    "data": "P2W",
    "description": "two weeks",
    "valid": true
  },
  {
    "data": "P1Y2W",
    "description": "weeks cannot be combined with other units",
    "valid": false
  },
  {
    "data": "P২Y",
    "description": "invalid non-ASCII '২' (a Bengali 2)",
    "valid": false
  },
  {
    "data": "P1",
    "description": "element without unit",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "duration"
}
"""

_VALIDATE_FORMATS = False

class Duration0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

