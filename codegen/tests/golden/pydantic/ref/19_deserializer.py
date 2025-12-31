from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, TypeAdapter
from pydantic.functional_validators import BeforeValidator

_VALIDATE_FORMATS = False

class Ref19Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$comment": "$id must be evaluated before $ref to get the proper $ref destination",
  "$defs": {
    "bigint": {
      "$anchor": "bigint",
      "$comment": "canonical uri: /ref-and-id2/base.json#/$defs/bigint; another valid uri for this location: /ref-and-id2/base.json#bigint",
      "maximum": 10
    },
    "smallint": {
      "$anchor": "bigint",
      "$comment": "canonical uri: https://example.com/ref-and-id2#/$defs/smallint; another valid uri for this location: https://example.com/ref-and-id2/#bigint",
      "$id": "https://example.com/draft2020-12/ref-and-id2/",
      "maximum": 2
    }
  },
  "$id": "https://example.com/draft2020-12/ref-and-id2/base.json",
  "$ref": "#bigint",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: Any

