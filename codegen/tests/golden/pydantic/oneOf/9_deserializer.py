from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class ModelDeserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "__root__": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "oneOf": [
        {
          "properties": {
            "bar": true,
            "baz": true
          },
          "required": [
            "bar"
          ]
        },
        {
          "properties": {
            "foo": true
          },
          "required": [
            "foo"
          ]
        }
      ]
    }
  },
  "$ref": "#/$defs/__root__/oneOf/0",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    model_config = ConfigDict(extra="allow")
    bar: Any
    baz: Annotated[Any | None, Field(default=None)]

class Model2Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "__root__": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "oneOf": [
        {
          "properties": {
            "bar": true,
            "baz": true
          },
          "required": [
            "bar"
          ]
        },
        {
          "properties": {
            "foo": true
          },
          "required": [
            "foo"
          ]
        }
      ]
    }
  },
  "$ref": "#/$defs/__root__/oneOf/1",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Any

class Oneof9Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "properties": {
        "bar": true,
        "baz": true
      },
      "required": [
        "bar"
      ]
    },
    {
      "properties": {
        "foo": true
      },
      "required": [
        "foo"
      ]
    }
  ]
}
"""
    root: ModelDeserializer | Model2Deserializer

