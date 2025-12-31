"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "__proto__": {
      "type": "number"
    },
    "constructor": {
      "type": "number"
    },
    "toString": {
      "properties": {
        "length": {
          "type": "string"
        }
      }
    }
  }
}

Tests:
[
  {
    "data": [],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
  },
  {
    "data": {},
    "description": "none of the properties mentioned",
    "valid": true
  },
  {
    "data": {
      "__proto__": "foo"
    },
    "description": "__proto__ not valid",
    "valid": false
  },
  {
    "data": {
      "toString": {
        "length": 37
      }
    },
    "description": "toString not valid",
    "valid": false
  },
  {
    "data": {
      "constructor": {
        "length": 37
      }
    },
    "description": "constructor not valid",
    "valid": false
  },
  {
    "data": {
      "__proto__": 12,
      "constructor": 37,
      "toString": {
        "length": "foo"
      }
    },
    "description": "all present and valid",
    "valid": true
  }
]
"""

from typing import Annotated, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "__proto__": {
      "type": "number"
    },
    "constructor": {
      "type": "number"
    },
    "toString": {
      "properties": {
        "length": {
          "type": "string"
        }
      }
    }
  }
}
"""

_VALIDATE_FORMATS = False

class ModelSerializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "__root__": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "properties": {
        "__proto__": {
          "type": "number"
        },
        "constructor": {
          "type": "number"
        },
        "toString": {
          "properties": {
            "length": {
              "type": "string"
            }
          }
        }
      }
    }
  },
  "$ref": "#/$defs/__root__/properties/toString",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    length: Annotated[str | None, Field(default=None)]

class Properties5Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    proto: Annotated[float | None, Field(alias="__proto__", default=None)]
    constructor: Annotated[float | None, Field(default=None)]
    to_string: Annotated[ModelSerializer | None, Field(alias="toString", default=None)]

