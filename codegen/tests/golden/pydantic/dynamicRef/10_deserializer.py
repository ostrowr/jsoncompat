"""
Schema:
{
  "$defs": {
    "bar": {
      "$id": "bar",
      "properties": {
        "baz": {
          "$dynamicRef": "extended#meta"
        }
      },
      "type": "object"
    },
    "extended": {
      "$anchor": "meta",
      "$id": "extended",
      "properties": {
        "bar": {
          "$ref": "bar"
        }
      },
      "type": "object"
    }
  },
  "$dynamicAnchor": "meta",
  "$id": "https://test.json-schema.org/relative-dynamic-reference-without-bookend/root",
  "$ref": "extended",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "const": "pass"
    }
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "bar": {
        "baz": {
          "foo": "fail"
        }
      },
      "foo": "pass"
    },
    "description": "The recursive part doesn't need to validate against the root",
    "valid": true
  }
]
"""

from typing import Annotated, Any, ClassVar, Literal

from json_schema_codegen_base import DeserializerBase, SerializerBase, _validate_literal
from pydantic import ConfigDict, Field, model_validator
from pydantic.functional_validators import BeforeValidator
from pydantic_core import core_schema

_JSON_SCHEMA = r"""
{
  "$defs": {
    "bar": {
      "$id": "bar",
      "properties": {
        "baz": {
          "$dynamicRef": "extended#meta"
        }
      },
      "type": "object"
    },
    "extended": {
      "$anchor": "meta",
      "$id": "extended",
      "properties": {
        "bar": {
          "$ref": "bar"
        }
      },
      "type": "object"
    }
  },
  "$dynamicAnchor": "meta",
  "$id": "https://test.json-schema.org/relative-dynamic-reference-without-bookend/root",
  "$ref": "extended",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "const": "pass"
    }
  },
  "type": "object"
}
"""

_VALIDATE_FORMATS = False

class ModelDeserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "bar": {
      "$id": "bar",
      "properties": {
        "baz": {
          "$dynamicRef": "extended#meta"
        }
      },
      "type": "object"
    },
    "extended": {
      "$anchor": "meta",
      "$id": "extended",
      "properties": {
        "bar": {
          "$ref": "bar"
        }
      },
      "type": "object"
    }
  },
  "$dynamicAnchor": "meta",
  "$id": "https://test.json-schema.org/relative-dynamic-reference-without-bookend/root",
  "$ref": "extended",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "const": "pass"
    }
  },
  "type": "object"
}
"""
    model_config = ConfigDict(extra="allow")
    baz: Annotated[Any | None, Field(default=None)]

class Dynamicref10Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    bar: Annotated[ModelDeserializer | None, Field(default=None)]
    foo: Annotated[Literal["pass"] | None, Field(default=None)]

