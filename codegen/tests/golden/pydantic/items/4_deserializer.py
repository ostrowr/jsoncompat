"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "items": {
      "items": {
        "items": {
          "type": "number"
        },
        "type": "array"
      },
      "type": "array"
    },
    "type": "array"
  },
  "type": "array"
}

Tests:
[
  {
    "data": [
      [
        [
          [
            1
          ]
        ],
        [
          [
            2
          ],
          [
            3
          ]
        ]
      ],
      [
        [
          [
            4
          ],
          [
            5
          ],
          [
            6
          ]
        ]
      ]
    ],
    "description": "valid nested array",
    "valid": true
  },
  {
    "data": [
      [
        [
          [
            "1"
          ]
        ],
        [
          [
            2
          ],
          [
            3
          ]
        ]
      ],
      [
        [
          [
            4
          ],
          [
            5
          ],
          [
            6
          ]
        ]
      ]
    ],
    "description": "nested array with invalid type",
    "valid": false
  },
  {
    "data": [
      [
        [
          1
        ],
        [
          2
        ],
        [
          3
        ]
      ],
      [
        [
          4
        ],
        [
          5
        ],
        [
          6
        ]
      ]
    ],
    "description": "not deep enough",
    "valid": false
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "items": {
      "items": {
        "items": {
          "type": "number"
        },
        "type": "array"
      },
      "type": "array"
    },
    "type": "array"
  },
  "type": "array"
}
"""

_VALIDATE_FORMATS = False

class Items4Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: list[list[list[list[float]]]]

