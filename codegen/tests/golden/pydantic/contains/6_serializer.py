"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "type": "null"
  }
}

Tests:
[
  {
    "data": [
      null
    ],
    "description": "allows null items",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Contains6Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")
