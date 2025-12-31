from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict
from pydantic.functional_validators import BeforeValidator

_VALIDATE_FORMATS = False

class Anchor0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "anchor_in_enum": {
      "enum": [
        {
          "$anchor": "my_anchor",
          "type": "null"
        }
      ]
    },
    "real_identifier_in_schema": {
      "$anchor": "my_anchor",
      "type": "string"
    },
    "zzz_anchor_in_const": {
      "const": {
        "$anchor": "my_anchor",
        "type": "null"
      }
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/anchor_in_enum"
    },
    {
      "$ref": "#my_anchor"
    }
  ]
}
"""
    root: Any | None

