from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Properties3Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo\tbar": {
      "type": "number"
    },
    "foo\nbar": {
      "type": "number"
    },
    "foo\fbar": {
      "type": "number"
    },
    "foo\rbar": {
      "type": "number"
    },
    "foo\"bar": {
      "type": "number"
    },
    "foo\\bar": {
      "type": "number"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    foo_bar: Annotated[float | None, Field(alias="foo\tbar", default=None)]
    foo_bar_2: Annotated[float | None, Field(alias="foo\nbar", default=None)]
    foo_bar_3: Annotated[float | None, Field(alias="foo\fbar", default=None)]
    foo_bar_4: Annotated[float | None, Field(alias="foo\rbar", default=None)]
    foo_bar_5: Annotated[float | None, Field(alias="foo\"bar", default=None)]
    foo_bar_6: Annotated[float | None, Field(alias="foo\\bar", default=None)]

