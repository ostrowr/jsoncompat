from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class ModelSerializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "__root__": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "properties": {
        "__proto__": {
          "type": "number"
        },
        "constructor": {
          "type": "number"
        },
        "toString": {
          "properties": {
            "length": {
              "type": "string"
            }
          }
        }
      }
    }
  },
  "$ref": "#/$defs/__root__/properties/toString",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    model_config = ConfigDict(extra="allow")
    length: Annotated[str | None, Field(default=None)]

class Properties5Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "__proto__": {
      "type": "number"
    },
    "constructor": {
      "type": "number"
    },
    "toString": {
      "properties": {
        "length": {
          "type": "string"
        }
      }
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    proto: Annotated[float | None, Field(alias="__proto__", default=None)]
    constructor: Annotated[float | None, Field(default=None)]
    to_string: Annotated[ModelSerializer | None, Field(alias="toString", default=None)]

