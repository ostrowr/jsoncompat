"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    false,
    false
  ]
}

Tests:
[
  {
    "data": "foo",
    "description": "any value is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Allof5Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")
