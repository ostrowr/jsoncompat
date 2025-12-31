"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "contains": {
      "const": "a"
    }
  },
  "then": {
    "if": {
      "contains": {
        "const": "b"
      }
    },
    "then": {
      "if": {
        "contains": {
          "const": "c"
        }
      }
    }
  },
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [],
    "description": "empty array is valid",
    "valid": true
  },
  {
    "data": [
      "a",
      "a"
    ],
    "description": "only a's are valid",
    "valid": true
  },
  {
    "data": [
      "a",
      "b",
      "a",
      "b",
      "a"
    ],
    "description": "a's and b's are valid",
    "valid": true
  },
  {
    "data": [
      "c",
      "a",
      "c",
      "c",
      "b",
      "a"
    ],
    "description": "a's, b's and c's are valid",
    "valid": true
  },
  {
    "data": [
      "b",
      "b"
    ],
    "description": "only b's are invalid",
    "valid": false
  },
  {
    "data": [
      "c",
      "c"
    ],
    "description": "only c's are invalid",
    "valid": false
  },
  {
    "data": [
      "c",
      "b",
      "c",
      "b",
      "c"
    ],
    "description": "only b's and c's are invalid",
    "valid": false
  },
  {
    "data": [
      "c",
      "a",
      "c",
      "a",
      "c"
    ],
    "description": "only a's and c's are invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems23Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")
