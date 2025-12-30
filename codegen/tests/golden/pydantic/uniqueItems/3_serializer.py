"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "uniqueItems": false
}

Tests:
[
  {
    "data": [
      1,
      2
    ],
    "description": "unique array of integers is valid",
    "valid": true
  },
  {
    "data": [
      1,
      1
    ],
    "description": "non-unique array of integers is valid",
    "valid": true
  },
  {
    "data": [
      1.0,
      1.0,
      1
    ],
    "description": "numbers are unique if mathematically unequal",
    "valid": true
  },
  {
    "data": [
      0,
      false
    ],
    "description": "false is not equal to zero",
    "valid": true
  },
  {
    "data": [
      1,
      true
    ],
    "description": "true is not equal to one",
    "valid": true
  },
  {
    "data": [
      {
        "foo": "bar"
      },
      {
        "foo": "baz"
      }
    ],
    "description": "unique array of objects is valid",
    "valid": true
  },
  {
    "data": [
      {
        "foo": "bar"
      },
      {
        "foo": "bar"
      }
    ],
    "description": "non-unique array of objects is valid",
    "valid": true
  },
  {
    "data": [
      {
        "foo": {
          "bar": {
            "baz": true
          }
        }
      },
      {
        "foo": {
          "bar": {
            "baz": false
          }
        }
      }
    ],
    "description": "unique array of nested objects is valid",
    "valid": true
  },
  {
    "data": [
      {
        "foo": {
          "bar": {
            "baz": true
          }
        }
      },
      {
        "foo": {
          "bar": {
            "baz": true
          }
        }
      }
    ],
    "description": "non-unique array of nested objects is valid",
    "valid": true
  },
  {
    "data": [
      [
        "foo"
      ],
      [
        "bar"
      ]
    ],
    "description": "unique array of arrays is valid",
    "valid": true
  },
  {
    "data": [
      [
        "foo"
      ],
      [
        "foo"
      ]
    ],
    "description": "non-unique array of arrays is valid",
    "valid": true
  },
  {
    "data": [
      1,
      true
    ],
    "description": "1 and true are unique",
    "valid": true
  },
  {
    "data": [
      0,
      false
    ],
    "description": "0 and false are unique",
    "valid": true
  },
  {
    "data": [
      {},
      [
        1
      ],
      true,
      null,
      1
    ],
    "description": "unique heterogeneous types are valid",
    "valid": true
  },
  {
    "data": [
      {},
      [
        1
      ],
      true,
      null,
      {},
      1
    ],
    "description": "non-unique heterogeneous types are valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Uniqueitems3Serializer(SerializerRootModel):
    root: Any

