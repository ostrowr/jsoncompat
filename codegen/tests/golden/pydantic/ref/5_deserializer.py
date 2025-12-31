"""
Schema:
{
  "$defs": {
    "reffed": {
      "type": "array"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/reffed",
      "maxItems": 2
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": []
    },
    "description": "ref valid, maxItems valid",
    "valid": true
  },
  {
    "data": {
      "foo": [
        1,
        2,
        3
      ]
    },
    "description": "ref valid, maxItems invalid",
    "valid": false
  },
  {
    "data": {
      "foo": "string"
    },
    "description": "ref invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Ref5Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/properties/foo/allOf/0: allOf with non-object schema")
