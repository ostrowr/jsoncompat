from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluatedproperties13Deserializer(DeserializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "properties": {
      "baz": {
        "type": "string"
      }
    },
    "required": [
      "baz"
    ]
  },
  "if": {
    "properties": {
      "foo": {
        "const": "then"
      }
    },
    "required": [
      "foo"
    ]
  },
  "then": {
    "properties": {
      "bar": {
        "type": "string"
      }
    },
    "required": [
      "bar"
    ]
  },
  "type": "object",
  "unevaluatedProperties": false
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/1: allOf with non-object schema"
