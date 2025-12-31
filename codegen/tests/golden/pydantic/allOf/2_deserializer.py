"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "maximum": 30
    },
    {
      "minimum": 20
    }
  ]
}

Tests:
[
  {
    "data": 25,
    "description": "valid",
    "valid": true
  },
  {
    "data": 35,
    "description": "mismatch one",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Allof2Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")
