from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluatedproperties7Deserializer(DeserializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "patternProperties": {
        "^bar": {
          "type": "string"
        }
      }
    }
  ],
  "properties": {
    "foo": {
      "type": "string"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/1: allOf with non-object schema"
