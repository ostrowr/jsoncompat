from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Unevaluateditems18Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "baseSchema": {
      "$comment": "unevaluatedItems comes first so it's more likely to catch bugs with implementations that are sensitive to keyword ordering",
      "$defs": {
        "defaultAddons": {
          "$comment": "Needed to satisfy the bookending requirement",
          "$dynamicAnchor": "addons"
        }
      },
      "$dynamicRef": "#addons",
      "$id": "./baseSchema",
      "prefixItems": [
        {
          "type": "string"
        }
      ],
      "type": "array",
      "unevaluatedItems": false
    },
    "derived": {
      "$dynamicAnchor": "addons",
      "prefixItems": [
        true,
        {
          "type": "string"
        }
      ]
    }
  },
  "$id": "https://example.com/unevaluated-items-with-dynamic-ref/derived",
  "$ref": "./baseSchema",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: list[str]

