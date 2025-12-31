from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Dependentschemas2Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentSchemas": {
    "foo\tbar": {
      "minProperties": 4
    },
    "foo'bar": {
      "required": [
        "foo\"bar"
      ]
    }
  }
}
"""
    root: Any

