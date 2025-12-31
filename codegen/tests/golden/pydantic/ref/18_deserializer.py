from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic.functional_validators import BeforeValidator

_VALIDATE_FORMATS = False

class Ref18Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$comment": "$id must be evaluated before $ref to get the proper $ref destination",
  "$defs": {
    "bigint": {
      "$comment": "canonical uri: https://example.com/ref-and-id1/int.json",
      "$id": "int.json",
      "maximum": 10
    },
    "smallint": {
      "$comment": "canonical uri: https://example.com/ref-and-id1-int.json",
      "$id": "/draft2020-12/ref-and-id1-int.json",
      "maximum": 2
    }
  },
  "$id": "https://example.com/draft2020-12/ref-and-id1/base.json",
  "$ref": "int.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: Any

