from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Patternproperties2Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "X_": {
      "type": "string"
    },
    "[0-9]{2,}": {
      "type": "boolean"
    }
  }
}
"""
    root: Any

