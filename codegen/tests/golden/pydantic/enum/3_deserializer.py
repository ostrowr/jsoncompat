from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

_VALIDATE_FORMATS = False

class Enum3Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "enum": [
        "bar"
      ]
    },
    "foo": {
      "enum": [
        "foo"
      ]
    }
  },
  "required": [
    "bar"
  ],
  "type": "object"
}
"""
    model_config = ConfigDict(extra="allow")
    bar: Literal["bar"]
    foo: Annotated[Literal["foo"] | None, Field(default=None)]

