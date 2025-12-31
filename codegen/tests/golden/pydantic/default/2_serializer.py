from typing import Annotated, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Default2Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "alpha": {
      "default": 5,
      "maximum": 3,
      "type": "number"
    }
  },
  "type": "object"
}
"""
    model_config = ConfigDict(extra="allow")
    alpha: Annotated[float | None, Field(default=None)]

