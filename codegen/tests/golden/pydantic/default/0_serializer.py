"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "default": [],
      "type": "integer"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": 13
    },
    "description": "valid when property is specified",
    "valid": true
  },
  {
    "data": {},
    "description": "still valid when the invalid default is used",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Default0Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("default value at #/properties/foo does not match the schema: default value [] does not match Integer(NumberConstraints { minimum: None, maximum: None, exclusive_minimum: false, exclusive_maximum: false, multiple_of: None, type_enforced: true })")
