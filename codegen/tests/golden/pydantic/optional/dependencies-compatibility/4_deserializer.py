from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Dependenciescompatibility4Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependencies": {
    "bar": {
      "properties": {
        "bar": {
          "type": "integer"
        },
        "foo": {
          "type": "integer"
        }
      }
    }
  }
}
"""
    root: Any

