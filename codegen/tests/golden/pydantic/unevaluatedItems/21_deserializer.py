"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "type": "string"
  },
  "prefixItems": [
    true
  ],
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      1,
      "foo"
    ],
    "description": "second item is evaluated by contains",
    "valid": true
  },
  {
    "data": [
      1,
      2
    ],
    "description": "contains fails, second item is not evaluated",
    "valid": false
  },
  {
    "data": [
      1,
      2,
      "foo"
    ],
    "description": "contains passes, second item is not evaluated",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems21Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")
