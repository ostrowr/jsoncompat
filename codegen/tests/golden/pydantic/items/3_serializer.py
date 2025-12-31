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

from typing import ClassVar

from jsonschema_rs import validator_for
from pydantic import BaseModel, ConfigDict, model_validator

_JSON_SCHEMA = r"""
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
"""
_VALIDATE_FORMATS = False

class Items3Serializer(BaseModel):
    __json_schema__: ClassVar[str] = _JSON_SCHEMA
    _jsonschema_validator: ClassVar[object | None] = None

    @classmethod
    def _get_jsonschema_validator(cls):
        validator = cls._jsonschema_validator
        if validator is None:
            validator = validator_for(cls.__json_schema__, validate_formats=_VALIDATE_FORMATS)
            cls._jsonschema_validator = validator
        return validator

    @model_validator(mode="before")
    @classmethod
    def _validate_jsonschema(cls, value):
        cls._get_jsonschema_validator().validate(value)
        return value

    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #: prefixItems/contains"
