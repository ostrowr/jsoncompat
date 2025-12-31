from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Ref23Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$comment": "RFC 8141 §2.3.1",
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:example:foo-bar-baz-qux?+CCResolve:cc=uk",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/bar"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]

