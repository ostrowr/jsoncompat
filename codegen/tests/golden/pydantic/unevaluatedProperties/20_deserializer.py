from typing import Annotated, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Unevaluatedproperties20Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "baseSchema": {
      "$comment": "unevaluatedProperties comes first so it's more likely to catch bugs with implementations that are sensitive to keyword ordering",
      "$defs": {
        "defaultAddons": {
          "$comment": "Needed to satisfy the bookending requirement",
          "$dynamicAnchor": "addons"
        }
      },
      "$dynamicRef": "#addons",
      "$id": "./baseSchema",
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "type": "object",
      "unevaluatedProperties": false
    },
    "derived": {
      "$dynamicAnchor": "addons",
      "properties": {
        "bar": {
          "type": "string"
        }
      }
    }
  },
  "$id": "https://example.com/unevaluated-properties-with-dynamic-ref/derived",
  "$ref": "./baseSchema",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]

