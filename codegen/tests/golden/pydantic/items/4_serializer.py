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

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Items4Serializer(SerializerRootModel):
    root: list[list[list[list[float]]]]

