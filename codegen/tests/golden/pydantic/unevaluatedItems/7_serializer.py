from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluateditems7Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "prefixItems": [
        true,
        {
          "type": "number"
        }
      ]
    }
  ],
  "prefixItems": [
    {
      "type": "string"
    }
  ],
  "unevaluatedItems": false
}
"""
    root: Any

