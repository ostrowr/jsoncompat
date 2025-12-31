from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Properties1Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "integer"
  },
  "patternProperties": {
    "f.o": {
      "minItems": 2
    }
  },
  "properties": {
    "bar": {
      "type": "array"
    },
    "foo": {
      "maxItems": 3,
      "type": "array"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    bar: Annotated[list[Any] | None, Field(default=None)]
    foo: Annotated[list[Any] | None, Field(default=None)]

