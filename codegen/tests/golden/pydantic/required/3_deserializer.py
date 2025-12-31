from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import core_schema

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

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo_bar: Annotated[Any, Field(alias="foo\tbar")]
    foo_bar_2: Annotated[Any, Field(alias="foo\nbar")]
    foo_bar_3: Annotated[Any, Field(alias="foo\fbar")]
    foo_bar_4: Annotated[Any, Field(alias="foo\rbar")]
    foo_bar_5: Annotated[Any, Field(alias="foo\"bar")]
    foo_bar_6: Annotated[Any, Field(alias="foo\\bar")]

