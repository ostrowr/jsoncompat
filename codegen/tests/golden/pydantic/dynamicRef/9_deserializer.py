from typing import Annotated, Any, Literal

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

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
      "$dynamicAnchor": "meta",
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
  "$id": "https://test.json-schema.org/relative-dynamic-reference/root",
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

class Dynamicref9Deserializer(DeserializerBase):
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
      "$dynamicAnchor": "meta",
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
  "$id": "https://test.json-schema.org/relative-dynamic-reference/root",
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
    bar: Annotated[ModelDeserializer | None, Field(default=None)]
    foo: Annotated[Literal["pass"] | None, Field(default=None)]

