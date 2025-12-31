from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

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
            "bar": {
              "type": "integer"
            }
          },
          "required": [
            "bar"
          ]
        },
        {
          "properties": {
            "foo": {
              "type": "string"
            }
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
    bar: int

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
            "bar": {
              "type": "integer"
            }
          },
          "required": [
            "bar"
          ]
        },
        {
          "properties": {
            "foo": {
              "type": "string"
            }
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
    foo: str

class Oneof6Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "properties": {
        "bar": {
          "type": "integer"
        }
      },
      "required": [
        "bar"
      ]
    },
    {
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "required": [
        "foo"
      ]
    }
  ]
}
"""
    root: ModelDeserializer | Model2Deserializer

