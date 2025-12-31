from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict
from pydantic.functional_validators import BeforeValidator

_VALIDATE_FORMATS = False

class Id0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "id_in_enum": {
      "enum": [
        {
          "$id": "https://localhost:1234/draft2020-12/id/my_identifier.json",
          "type": "null"
        }
      ]
    },
    "real_id_in_schema": {
      "$id": "https://localhost:1234/draft2020-12/id/my_identifier.json",
      "type": "string"
    },
    "zzz_id_in_const": {
      "const": {
        "$id": "https://localhost:1234/draft2020-12/id/my_identifier.json",
        "type": "null"
      }
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/id_in_enum"
    },
    {
      "$ref": "https://localhost:1234/draft2020-12/id/my_identifier.json"
    }
  ]
}
"""
    root: Any | str

