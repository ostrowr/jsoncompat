"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "integer"
  },
  "patternProperties": {
    "f.o": {
      "minItems": 2
    }
  },
  "properties": {
    "bar": {
      "type": "array"
    },
    "foo": {
      "maxItems": 3,
      "type": "array"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": [
        1,
        2
      ]
    },
    "description": "property validates property",
    "valid": true
  },
  {
    "data": {
      "foo": [
        1,
        2,
        3,
        4
      ]
    },
    "description": "property invalidates property",
    "valid": false
  },
  {
    "data": {
      "foo": []
    },
    "description": "patternProperty invalidates property",
    "valid": false
  },
  {
    "data": {
      "fxo": [
        1,
        2
      ]
    },
    "description": "patternProperty validates nonproperty",
    "valid": true
  },
  {
    "data": {
      "fxo": []
    },
    "description": "patternProperty invalidates nonproperty",
    "valid": false
  },
  {
    "data": {
      "bar": []
    },
    "description": "additionalProperty ignores property",
    "valid": true
  },
  {
    "data": {
      "quux": 3
    },
    "description": "additionalProperty validates others",
    "valid": true
  },
  {
    "data": {
      "quux": "foo"
    },
    "description": "additionalProperty invalidates others",
    "valid": false
  }
]
"""

from typing import Annotated, Any, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "integer"
  },
  "patternProperties": {
    "f.o": {
      "minItems": 2
    }
  },
  "properties": {
    "bar": {
      "type": "array"
    },
    "foo": {
      "maxItems": 3,
      "type": "array"
    }
  }
}
"""

_VALIDATE_FORMATS = False

class Properties1Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    bar: Annotated[list[Any] | None, Field(default=None)]
    foo: Annotated[list[Any] | None, Field(default=None)]

