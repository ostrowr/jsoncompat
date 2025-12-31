from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Refremote10Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$id": "http://localhost:1234/draft2020-12/some-id",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "name": {
      "$ref": "nested/foo-ref-string.json"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    name: Annotated[Any | None, Field(default=None)]

