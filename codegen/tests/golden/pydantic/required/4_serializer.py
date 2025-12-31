from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Required4Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "required": [
    "__proto__",
    "toString",
    "constructor"
  ]
}
"""
    model_config = ConfigDict(extra="allow")
    proto: Annotated[Any, Field(alias="__proto__")]
    constructor: Any
    to_string: Annotated[Any, Field(alias="toString")]

