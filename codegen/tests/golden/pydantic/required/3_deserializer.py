from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Required3Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "required": [
    "foo\nbar",
    "foo\"bar",
    "foo\\bar",
    "foo\rbar",
    "foo\tbar",
    "foo\fbar"
  ]
}
"""
    model_config = ConfigDict(extra="allow")
    foo_bar: Annotated[Any, Field(alias="foo\tbar")]
    foo_bar_2: Annotated[Any, Field(alias="foo\nbar")]
    foo_bar_3: Annotated[Any, Field(alias="foo\fbar")]
    foo_bar_4: Annotated[Any, Field(alias="foo\rbar")]
    foo_bar_5: Annotated[Any, Field(alias="foo\"bar")]
    foo_bar_6: Annotated[Any, Field(alias="foo\\bar")]

