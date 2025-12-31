"""
Schema:
{
  "$defs": {
    "A": {
      "unevaluatedProperties": false
    }
  },
  "$ref": "#/$defs/A",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "prop1": {
      "type": "string"
    }
  }
}

Tests:
[
  {
    "data": {
      "prop1": "match"
    },
    "description": "referenced subschema doesn't see annotations from properties",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Ref13Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")
