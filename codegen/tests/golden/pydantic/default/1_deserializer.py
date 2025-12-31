from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Default1Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "default": "bad",
      "minLength": 4,
      "type": "string"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    bar: Annotated[str | None, Field(default="bad")]

