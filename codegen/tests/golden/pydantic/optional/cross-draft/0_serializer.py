"""
Schema:
{
  "$ref": "http://localhost:1234/draft2019-09/ignore-prefixItems.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "array"
}

Tests:
[
  {
    "comment": "if the implementation is not processing the $ref as a 2019-09 schema, this test will fail",
    "data": [
      1,
      2,
      3
    ],
    "description": "first item not a string is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class CrossDraft0Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")
