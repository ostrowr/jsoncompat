"""
Schema:
{
  "$dynamicAnchor": "node",
  "$id": "http://localhost:1234/draft2020-12/strict-tree.json",
  "$ref": "tree.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {
      "children": [
        {
          "daat": 1
        }
      ]
    },
    "description": "instance with misspelled field",
    "valid": false
  },
  {
    "data": {
      "children": [
        {
          "data": 1
        }
      ]
    },
    "description": "instance with correct field",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Dynamicref13Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")
