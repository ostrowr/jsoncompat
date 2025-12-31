from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluateditems18Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
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
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #: prefixItems/contains"
