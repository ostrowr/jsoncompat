from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Dynamicref13Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$dynamicAnchor": "node",
  "$id": "http://localhost:1234/draft2020-12/strict-tree.json",
  "$ref": "tree.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedProperties": false
}
"""
    root: Any

