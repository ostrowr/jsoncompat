"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "uuid"
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
    "data": "2EB8AA08-AA98-11EA-B4AA-73B441D16380",
    "description": "all upper-case",
    "valid": true
  },
  {
    "data": "2eb8aa08-aa98-11ea-b4aa-73b441d16380",
    "description": "all lower-case",
    "valid": true
  },
  {
    "data": "2eb8aa08-AA98-11ea-B4Aa-73B441D16380",
    "description": "mixed case",
    "valid": true
  },
  {
    "data": "00000000-0000-0000-0000-000000000000",
    "description": "all zeroes is valid",
    "valid": true
  },
  {
    "data": "2eb8aa08-aa98-11ea-b4aa-73b441d1638",
    "description": "wrong length",
    "valid": false
  },
  {
    "data": "2eb8aa08-aa98-11ea-73b441d16380",
    "description": "missing section",
    "valid": false
  },
  {
    "data": "2eb8aa08-aa98-11ea-b4ga-73b441d16380",
    "description": "bad characters (not hex)",
    "valid": false
  },
  {
    "data": "2eb8aa08aa9811eab4aa73b441d16380",
    "description": "no dashes",
    "valid": false
  },
  {
    "data": "2eb8aa08aa98-11ea-b4aa73b441d16380",
    "description": "too few dashes",
    "valid": false
  },
  {
    "data": "2eb8-aa08-aa98-11ea-b4aa73b44-1d16380",
    "description": "too many dashes",
    "valid": false
  },
  {
    "data": "2eb8aa08aa9811eab4aa73b441d16380----",
    "description": "dashes in the wrong spot",
    "valid": false
  },
  {
    "data": "98d80576-482e-427f-8434-7f86890ab222",
    "description": "valid version 4",
    "valid": true
  },
  {
    "data": "99c17cbb-656f-564a-940f-1a4568f03487",
    "description": "valid version 5",
    "valid": true
  },
  {
    "data": "99c17cbb-656f-664a-940f-1a4568f03487",
    "description": "hypothetical version 6",
    "valid": true
  },
  {
    "data": "99c17cbb-656f-f64a-940f-1a4568f03487",
    "description": "hypothetical version 15",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Uuid0Deserializer(DeserializerRootModel):
    root: Any

