"""
Schema:
{
  "$defs": {
    "item": {
      "items": false,
      "prefixItems": [
        {
          "$ref": "#/$defs/sub-item"
        },
        {
          "$ref": "#/$defs/sub-item"
        }
      ],
      "type": "array"
    },
    "sub-item": {
      "required": [
        "foo"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": false,
  "prefixItems": [
    {
      "$ref": "#/$defs/item"
    },
    {
      "$ref": "#/$defs/item"
    },
    {
      "$ref": "#/$defs/item"
    }
  ],
  "type": "array"
}

Tests:
[
  {
    "data": [
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ]
    ],
    "description": "valid items",
    "valid": true
  },
  {
    "data": [
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ]
    ],
    "description": "too many items",
    "valid": false
  },
  {
    "data": [
      [
        {
          "foo": null
        },
        {
          "foo": null
        },
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ]
    ],
    "description": "too many sub-items",
    "valid": false
  },
  {
    "data": [
      {
        "foo": null
      },
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ]
    ],
    "description": "wrong item",
    "valid": false
  },
  {
    "data": [
      [
        {},
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        },
        {
          "foo": null
        }
      ]
    ],
    "description": "wrong sub-item",
    "valid": false
  },
  {
    "data": [
      [
        {
          "foo": null
        }
      ],
      [
        {
          "foo": null
        }
      ]
    ],
    "description": "fewer items is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Items3Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")
