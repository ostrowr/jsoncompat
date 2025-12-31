"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "date"
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
    "data": "1963-06-19",
    "description": "a valid date string",
    "valid": true
  },
  {
    "data": "2020-01-31",
    "description": "a valid date string with 31 days in January",
    "valid": true
  },
  {
    "data": "2020-01-32",
    "description": "a invalid date string with 32 days in January",
    "valid": false
  },
  {
    "data": "2021-02-28",
    "description": "a valid date string with 28 days in February (normal)",
    "valid": true
  },
  {
    "data": "2021-02-29",
    "description": "a invalid date string with 29 days in February (normal)",
    "valid": false
  },
  {
    "data": "2020-02-29",
    "description": "a valid date string with 29 days in February (leap)",
    "valid": true
  },
  {
    "data": "2020-02-30",
    "description": "a invalid date string with 30 days in February (leap)",
    "valid": false
  },
  {
    "data": "2020-03-31",
    "description": "a valid date string with 31 days in March",
    "valid": true
  },
  {
    "data": "2020-03-32",
    "description": "a invalid date string with 32 days in March",
    "valid": false
  },
  {
    "data": "2020-04-30",
    "description": "a valid date string with 30 days in April",
    "valid": true
  },
  {
    "data": "2020-04-31",
    "description": "a invalid date string with 31 days in April",
    "valid": false
  },
  {
    "data": "2020-05-31",
    "description": "a valid date string with 31 days in May",
    "valid": true
  },
  {
    "data": "2020-05-32",
    "description": "a invalid date string with 32 days in May",
    "valid": false
  },
  {
    "data": "2020-06-30",
    "description": "a valid date string with 30 days in June",
    "valid": true
  },
  {
    "data": "2020-06-31",
    "description": "a invalid date string with 31 days in June",
    "valid": false
  },
  {
    "data": "2020-07-31",
    "description": "a valid date string with 31 days in July",
    "valid": true
  },
  {
    "data": "2020-07-32",
    "description": "a invalid date string with 32 days in July",
    "valid": false
  },
  {
    "data": "2020-08-31",
    "description": "a valid date string with 31 days in August",
    "valid": true
  },
  {
    "data": "2020-08-32",
    "description": "a invalid date string with 32 days in August",
    "valid": false
  },
  {
    "data": "2020-09-30",
    "description": "a valid date string with 30 days in September",
    "valid": true
  },
  {
    "data": "2020-09-31",
    "description": "a invalid date string with 31 days in September",
    "valid": false
  },
  {
    "data": "2020-10-31",
    "description": "a valid date string with 31 days in October",
    "valid": true
  },
  {
    "data": "2020-10-32",
    "description": "a invalid date string with 32 days in October",
    "valid": false
  },
  {
    "data": "2020-11-30",
    "description": "a valid date string with 30 days in November",
    "valid": true
  },
  {
    "data": "2020-11-31",
    "description": "a invalid date string with 31 days in November",
    "valid": false
  },
  {
    "data": "2020-12-31",
    "description": "a valid date string with 31 days in December",
    "valid": true
  },
  {
    "data": "2020-12-32",
    "description": "a invalid date string with 32 days in December",
    "valid": false
  },
  {
    "data": "2020-13-01",
    "description": "a invalid date string with invalid month",
    "valid": false
  },
  {
    "data": "06/19/1963",
    "description": "an invalid date string",
    "valid": false
  },
  {
    "data": "2013-350",
    "description": "only RFC3339 not all of ISO 8601 are valid",
    "valid": false
  },
  {
    "data": "1998-1-20",
    "description": "non-padded month dates are not valid",
    "valid": false
  },
  {
    "data": "1998-01-1",
    "description": "non-padded day dates are not valid",
    "valid": false
  },
  {
    "data": "1998-13-01",
    "description": "invalid month",
    "valid": false
  },
  {
    "data": "1998-04-31",
    "description": "invalid month-day combination",
    "valid": false
  },
  {
    "data": "2021-02-29",
    "description": "2021 is not a leap year",
    "valid": false
  },
  {
    "data": "2020-02-29",
    "description": "2020 is a leap year",
    "valid": true
  },
  {
    "data": "1963-06-1৪",
    "description": "invalid non-ASCII '৪' (a Bengali 4)",
    "valid": false
  },
  {
    "data": "20230328",
    "description": "ISO8601 / non-RFC3339: YYYYMMDD without dashes (2023-03-28)",
    "valid": false
  },
  {
    "data": "2023-W01",
    "description": "ISO8601 / non-RFC3339: week number implicit day of week (2023-01-02)",
    "valid": false
  },
  {
    "data": "2023-W13-2",
    "description": "ISO8601 / non-RFC3339: week number with day of week (2023-03-28)",
    "valid": false
  },
  {
    "data": "2022W527",
    "description": "ISO8601 / non-RFC3339: week number rollover to next year (2023-01-01)",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Date0Deserializer(DeserializerRootModel):
    root: Any

