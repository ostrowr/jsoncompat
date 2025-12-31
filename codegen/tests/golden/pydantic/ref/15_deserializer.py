"""
Schema:
{
  "$id": "http://example.com/schema-relative-uri-defs1.json",
  "$ref": "schema-relative-uri-defs2.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$defs": {
        "inner": {
          "properties": {
            "bar": {
              "type": "string"
            }
          }
        }
      },
      "$id": "schema-relative-uri-defs2.json",
      "$ref": "#/$defs/inner"
    }
  }
}

Tests:
[
  {
    "data": {
      "bar": "a",
      "foo": {
        "bar": 1
      }
    },
    "description": "invalid on inner field",
    "valid": false
  },
  {
    "data": {
      "bar": 1,
      "foo": {
        "bar": "a"
      }
    },
    "description": "invalid on outer field",
    "valid": false
  },
  {
    "data": {
      "bar": "a",
      "foo": {
        "bar": "a"
      }
    },
    "description": "valid on both fields",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Ref15Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")
