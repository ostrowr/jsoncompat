"""
Schema:
{
  "$defs": {
    "id_in_unknown0": {
      "not": {
        "array_of_schemas": [
          {
            "$id": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json",
            "type": "null"
          }
        ]
      }
    },
    "id_in_unknown1": {
      "not": {
        "object_of_schemas": {
          "foo": {
            "$id": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json",
            "type": "integer"
          }
        }
      }
    },
    "real_id_in_schema": {
      "$id": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json",
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/id_in_unknown0"
    },
    {
      "$ref": "#/$defs/id_in_unknown1"
    },
    {
      "$ref": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json"
    }
  ]
}

Tests:
[
  {
    "data": "a string",
    "description": "type matches second anyOf, which has a real schema in it",
    "valid": true
  },
  {
    "data": null,
    "description": "type matches non-schema in first anyOf",
    "valid": false
  },
  {
    "data": 1,
    "description": "type matches non-schema in third anyOf",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$defs": {
    "id_in_unknown0": {
      "not": {
        "array_of_schemas": [
          {
            "$id": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json",
            "type": "null"
          }
        ]
      }
    },
    "id_in_unknown1": {
      "not": {
        "object_of_schemas": {
          "foo": {
            "$id": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json",
            "type": "integer"
          }
        }
      }
    },
    "real_id_in_schema": {
      "$id": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json",
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/id_in_unknown0"
    },
    {
      "$ref": "#/$defs/id_in_unknown1"
    },
    {
      "$ref": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json"
    }
  ]
}
"""

_VALIDATE_FORMATS = False

class Unknownkeyword0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any | str

