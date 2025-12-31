from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Dynamicref1Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "foo": {
      "$anchor": "items",
      "type": "string"
    }
  },
  "$id": "https://test.json-schema.org/dynamicRef-anchor-same-schema/root",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$dynamicRef": "#items"
  },
  "type": "array"
}
"""
    root: list[Any]

