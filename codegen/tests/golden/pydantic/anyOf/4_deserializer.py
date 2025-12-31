"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
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

class Anyof4Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/anyOf/0: false schema")
