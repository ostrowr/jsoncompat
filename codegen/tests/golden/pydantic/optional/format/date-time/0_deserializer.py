"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "date-time"
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
    "data": "1963-06-19T08:30:06.283185Z",
    "description": "a valid date-time string",
    "valid": true
  },
  {
    "data": "1963-06-19T08:30:06Z",
    "description": "a valid date-time string without second fraction",
    "valid": true
  },
  {
    "data": "1937-01-01T12:00:27.87+00:20",
    "description": "a valid date-time string with plus offset",
    "valid": true
  },
  {
    "data": "1990-12-31T15:59:50.123-08:00",
    "description": "a valid date-time string with minus offset",
    "valid": true
  },
  {
    "data": "1998-12-31T23:59:60Z",
    "description": "a valid date-time with a leap second, UTC",
    "valid": true
  },
  {
    "data": "1998-12-31T15:59:60.123-08:00",
    "description": "a valid date-time with a leap second, with minus offset",
    "valid": true
  },
  {
    "data": "1998-12-31T23:59:61Z",
    "description": "an invalid date-time past leap second, UTC",
    "valid": false
  },
  {
    "data": "1998-12-31T23:58:60Z",
    "description": "an invalid date-time with leap second on a wrong minute, UTC",
    "valid": false
  },
  {
    "data": "1998-12-31T22:59:60Z",
    "description": "an invalid date-time with leap second on a wrong hour, UTC",
    "valid": false
  },
  {
    "data": "1990-02-31T15:59:59.123-08:00",
    "description": "an invalid day in date-time string",
    "valid": false
  },
  {
    "data": "1990-12-31T15:59:59-24:00",
    "description": "an invalid offset in date-time string",
    "valid": false
  },
  {
    "data": "1963-06-19T08:30:06.28123+01:00Z",
    "description": "an invalid closing Z after time-zone offset",
    "valid": false
  },
  {
    "data": "06/19/1963 08:30:06 PST",
    "description": "an invalid date-time string",
    "valid": false
  },
  {
    "data": "1963-06-19t08:30:06.283185z",
    "description": "case-insensitive T and Z",
    "valid": true
  },
  {
    "data": "2013-350T01:01:01",
    "description": "only RFC3339 not all of ISO 8601 are valid",
    "valid": false
  },
  {
    "data": "1963-6-19T08:30:06.283185Z",
    "description": "invalid non-padded month dates",
    "valid": false
  },
  {
    "data": "1963-06-1T08:30:06.283185Z",
    "description": "invalid non-padded day dates",
    "valid": false
  },
  {
    "data": "1963-06-1৪T00:00:00Z",
    "description": "invalid non-ASCII '৪' (a Bengali 4) in date portion",
    "valid": false
  },
  {
    "data": "1963-06-11T0৪:00:00Z",
    "description": "invalid non-ASCII '৪' (a Bengali 4) in time portion",
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
  "format": "date-time"
}
"""

_VALIDATE_FORMATS = False

class Datetime0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

