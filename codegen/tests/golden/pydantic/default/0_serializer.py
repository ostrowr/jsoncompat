from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase, Impossible
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Default0Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "default": [],
      "type": "integer"
    }
  }
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "default value at #/properties/foo does not match the schema: default value [] does not match Integer(NumberConstraints { minimum: None, maximum: None, exclusive_minimum: false, exclusive_maximum: false, multiple_of: None, type_enforced: true })"
