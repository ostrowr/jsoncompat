"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "f.*o": {
      "type": "integer"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "a single valid match is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 1,
      "foooooo": 2
    },
    "description": "multiple valid matches is valid",
    "valid": true
  },
  {
    "data": {
      "foo": "bar",
      "fooooo": 2
    },
    "description": "a single invalid match is invalid",
    "valid": false
  },
  {
    "data": {
      "foo": "bar",
      "foooooo": "baz"
    },
    "description": "multiple invalid matches is invalid",
    "valid": false
  },
  {
    "data": [
      "foo"
    ],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": "foo",
    "description": "ignores strings",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Patternproperties0Deserializer(DeserializerRootModel):
    root: Any

