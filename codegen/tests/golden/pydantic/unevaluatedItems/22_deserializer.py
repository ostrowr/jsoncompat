from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluateditems22Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "contains": {
        "multipleOf": 2
      }
    },
    {
      "contains": {
        "multipleOf": 3
      }
    }
  ],
  "unevaluatedItems": {
    "multipleOf": 5
  }
}
"""
    root: Any

