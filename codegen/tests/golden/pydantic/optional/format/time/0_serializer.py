"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "time"
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
    "data": "08:30:06Z",
    "description": "a valid time string",
    "valid": true
  },
  {
    "data": "008:030:006Z",
    "description": "invalid time string with extra leading zeros",
    "valid": false
  },
  {
    "data": "8:3:6Z",
    "description": "invalid time string with no leading zero for single digit",
    "valid": false
  },
  {
    "data": "8:0030:6Z",
    "description": "hour, minute, second must be two digits",
    "valid": false
  },
  {
    "data": "23:59:60Z",
    "description": "a valid time string with leap second, Zulu",
    "valid": true
  },
  {
    "data": "22:59:60Z",
    "description": "invalid leap second, Zulu (wrong hour)",
    "valid": false
  },
  {
    "data": "23:58:60Z",
    "description": "invalid leap second, Zulu (wrong minute)",
    "valid": false
  },
  {
    "data": "23:59:60+00:00",
    "description": "valid leap second, zero time-offset",
    "valid": true
  },
  {
    "data": "22:59:60+00:00",
    "description": "invalid leap second, zero time-offset (wrong hour)",
    "valid": false
  },
  {
    "data": "23:58:60+00:00",
    "description": "invalid leap second, zero time-offset (wrong minute)",
    "valid": false
  },
  {
    "data": "01:29:60+01:30",
    "description": "valid leap second, positive time-offset",
    "valid": true
  },
  {
    "data": "23:29:60+23:30",
    "description": "valid leap second, large positive time-offset",
    "valid": true
  },
  {
    "data": "23:59:60+01:00",
    "description": "invalid leap second, positive time-offset (wrong hour)",
    "valid": false
  },
  {
    "data": "23:59:60+00:30",
    "description": "invalid leap second, positive time-offset (wrong minute)",
    "valid": false
  },
  {
    "data": "15:59:60-08:00",
    "description": "valid leap second, negative time-offset",
    "valid": true
  },
  {
    "data": "00:29:60-23:30",
    "description": "valid leap second, large negative time-offset",
    "valid": true
  },
  {
    "data": "23:59:60-01:00",
    "description": "invalid leap second, negative time-offset (wrong hour)",
    "valid": false
  },
  {
    "data": "23:59:60-00:30",
    "description": "invalid leap second, negative time-offset (wrong minute)",
    "valid": false
  },
  {
    "data": "23:20:50.52Z",
    "description": "a valid time string with second fraction",
    "valid": true
  },
  {
    "data": "08:30:06.283185Z",
    "description": "a valid time string with precise second fraction",
    "valid": true
  },
  {
    "data": "08:30:06+00:20",
    "description": "a valid time string with plus offset",
    "valid": true
  },
  {
    "data": "08:30:06-08:00",
    "description": "a valid time string with minus offset",
    "valid": true
  },
  {
    "data": "08:30:06-8:000",
    "description": "hour, minute in time-offset must be two digits",
    "valid": false
  },
  {
    "data": "08:30:06z",
    "description": "a valid time string with case-insensitive Z",
    "valid": true
  },
  {
    "data": "24:00:00Z",
    "description": "an invalid time string with invalid hour",
    "valid": false
  },
  {
    "data": "00:60:00Z",
    "description": "an invalid time string with invalid minute",
    "valid": false
  },
  {
    "data": "00:00:61Z",
    "description": "an invalid time string with invalid second",
    "valid": false
  },
  {
    "data": "22:59:60Z",
    "description": "an invalid time string with invalid leap second (wrong hour)",
    "valid": false
  },
  {
    "data": "23:58:60Z",
    "description": "an invalid time string with invalid leap second (wrong minute)",
    "valid": false
  },
  {
    "data": "01:02:03+24:00",
    "description": "an invalid time string with invalid time numoffset hour",
    "valid": false
  },
  {
    "data": "01:02:03+00:60",
    "description": "an invalid time string with invalid time numoffset minute",
    "valid": false
  },
  {
    "data": "01:02:03Z+00:30",
    "description": "an invalid time string with invalid time with both Z and numoffset",
    "valid": false
  },
  {
    "data": "08:30:06 PST",
    "description": "an invalid offset indicator",
    "valid": false
  },
  {
    "data": "01:01:01,1111",
    "description": "only RFC3339 not all of ISO 8601 are valid",
    "valid": false
  },
  {
    "data": "12:00:00",
    "description": "no time offset",
    "valid": false
  },
  {
    "data": "12:00:00.52",
    "description": "no time offset with second fraction",
    "valid": false
  },
  {
    "data": "1২:00:00Z",
    "description": "invalid non-ASCII '২' (a Bengali 2)",
    "valid": false
  },
  {
    "data": "08:30:06#00:20",
    "description": "offset not starting with plus or minus",
    "valid": false
  },
  {
    "data": "ab:cd:ef",
    "description": "contains letters",
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
  "format": "time"
}
"""

_VALIDATE_FORMATS = False

class Time0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

