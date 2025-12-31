from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluateditems14Deserializer(DeserializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "prefixItems": [
      true,
      true,
      true,
      {
        "const": "else"
      }
    ]
  },
  "if": {
    "prefixItems": [
      true,
      {
        "const": "bar"
      }
    ]
  },
  "prefixItems": [
    {
      "const": "foo"
    }
  ],
  "then": {
    "prefixItems": [
      true,
      true,
      {
        "const": "then"
      }
    ]
  },
  "unevaluatedItems": false
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/0: allOf with non-object schema"
