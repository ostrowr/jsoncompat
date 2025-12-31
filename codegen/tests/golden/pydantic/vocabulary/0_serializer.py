"""
Schema:
{
  "$id": "https://schema/using/no/validation",
  "$schema": "http://localhost:1234/draft2020-12/metaschema-no-validation.json",
  "properties": {
    "badProperty": false,
    "numberProperty": {
      "minimum": 10
    }
  }
}

Tests:
[
  {
    "data": {
      "badProperty": "this property should not exist"
    },
    "description": "applicator vocabulary still works",
    "valid": false
  },
  {
    "data": {
      "numberProperty": 20
    },
    "description": "no validation: valid number",
    "valid": true
  },
  {
    "data": {
      "numberProperty": 1
    },
    "description": "no validation: invalid number, but it still validates",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Vocabulary0Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/properties/badProperty: false schema")
