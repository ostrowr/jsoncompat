from typing import Annotated, Any, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Refremote7Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$id": "http://localhost:1234/draft2020-12/object",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "name": {
      "$ref": "name-defs.json#/$defs/orNull"
    }
  },
  "type": "object"
}
"""
    model_config = ConfigDict(extra="allow")
    name: Annotated[Any | None, Field(default=None)]

