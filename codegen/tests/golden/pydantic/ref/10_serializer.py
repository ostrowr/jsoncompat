"""
Schema:
{
  "$defs": {
    "bool": false
  },
  "$ref": "#/$defs/bool",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
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

class Ref10Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: false schema")
