from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Allof1Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "required": [
        "foo"
      ]
    },
    {
      "properties": {
        "baz": {
          "type": "null"
        }
      },
      "required": [
        "baz"
      ]
    }
  ],
  "properties": {
    "bar": {
      "type": "integer"
    }
  },
  "required": [
    "bar"
  ]
}
"""
    model_config = ConfigDict(extra="allow")
    bar: int | float
    baz: None
    foo: str

