from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, TypeAdapter
from pydantic.functional_validators import BeforeValidator

_VALIDATE_FORMATS = False

class Refremote4Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$id": "http://localhost:1234/draft2020-12/",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$id": "baseUriChange/",
    "items": {
      "$ref": "folderInteger.json"
    }
  }
}
"""
    root: Any

