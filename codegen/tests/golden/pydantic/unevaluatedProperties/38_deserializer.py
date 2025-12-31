"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "patternProperties": {
      "foo": {
        "type": "string"
      }
    }
  },
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {
      "foo": "a"
    },
    "description": "valid in case if is evaluated",
    "valid": true
  },
  {
    "data": {
      "bar": "a"
    },
    "description": "invalid in case if is evaluated",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluatedproperties38Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")
