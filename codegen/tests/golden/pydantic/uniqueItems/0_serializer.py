"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "uniqueItems": true
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
    "description": "non-unique array of integers is invalid",
    "valid": false
  },
  {
    "data": [
      1,
      2,
      1
    ],
    "description": "non-unique array of more than two integers is invalid",
    "valid": false
  },
  {
    "data": [
      1.0,
      1.0,
      1
    ],
    "description": "numbers are unique if mathematically unequal",
    "valid": false
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
      "foo",
      "bar",
      "baz"
    ],
    "description": "unique array of strings is valid",
    "valid": true
  },
  {
    "data": [
      "foo",
      "bar",
      "foo"
    ],
    "description": "non-unique array of strings is invalid",
    "valid": false
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
    "description": "non-unique array of objects is invalid",
    "valid": false
  },
  {
    "data": [
      {
        "bar": "foo",
        "foo": "bar"
      },
      {
        "bar": "foo",
        "foo": "bar"
      }
    ],
    "description": "property order of array of objects is ignored",
    "valid": false
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
    "description": "non-unique array of nested objects is invalid",
    "valid": false
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
    "description": "non-unique array of arrays is invalid",
    "valid": false
  },
  {
    "data": [
      [
        "foo"
      ],
      [
        "bar"
      ],
      [
        "foo"
      ]
    ],
    "description": "non-unique array of more than two arrays is invalid",
    "valid": false
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
      [
        1
      ],
      [
        true
      ]
    ],
    "description": "[1] and [true] are unique",
    "valid": true
  },
  {
    "data": [
      [
        0
      ],
      [
        false
      ]
    ],
    "description": "[0] and [false] are unique",
    "valid": true
  },
  {
    "data": [
      [
        [
          1
        ],
        "foo"
      ],
      [
        [
          true
        ],
        "foo"
      ]
    ],
    "description": "nested [1] and [true] are unique",
    "valid": true
  },
  {
    "data": [
      [
        [
          0
        ],
        "foo"
      ],
      [
        [
          false
        ],
        "foo"
      ]
    ],
    "description": "nested [0] and [false] are unique",
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
      1,
      "{}"
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
    "description": "non-unique heterogeneous types are invalid",
    "valid": false
  },
  {
    "data": [
      {
        "a": 1,
        "b": 2
      },
      {
        "a": 2,
        "b": 1
      }
    ],
    "description": "different objects are unique",
    "valid": true
  },
  {
    "data": [
      {
        "a": 1,
        "b": 2
      },
      {
        "a": 1,
        "b": 2
      }
    ],
    "description": "objects are non-unique despite key order",
    "valid": false
  },
  {
    "data": [
      {
        "a": false
      },
      {
        "a": 0
      }
    ],
    "description": "{\"a\": false} and {\"a\": 0} are unique",
    "valid": true
  },
  {
    "data": [
      {
        "a": true
      },
      {
        "a": 1
      }
    ],
    "description": "{\"a\": true} and {\"a\": 1} are unique",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Uniqueitems0Serializer(SerializerRootModel):
    root: Any

