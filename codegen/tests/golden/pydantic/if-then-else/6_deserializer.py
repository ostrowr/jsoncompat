"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "if": {
        "exclusiveMaximum": 0
      }
    },
    {
      "then": {
        "minimum": -10
      }
    },
    {
      "else": {
        "multipleOf": 2
      }
    }
  ]
}

Tests:
[
  {
    "data": -100,
    "description": "valid, but would have been invalid through then",
    "valid": true
  },
  {
    "data": 3,
    "description": "valid, but would have been invalid through else",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class IfThenElse6Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")
