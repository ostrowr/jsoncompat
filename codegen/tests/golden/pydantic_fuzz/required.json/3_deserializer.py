from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Required3Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo_bar: Annotated[Any, Field(alias="foo\tbar")]
    foo_bar_2: Annotated[Any, Field(alias="foo\nbar")]
    foo_bar_3: Annotated[Any, Field(alias="foo\fbar")]
    foo_bar_4: Annotated[Any, Field(alias="foo\rbar")]
    foo_bar_5: Annotated[Any, Field(alias="foo\"bar")]
    foo_bar_6: Annotated[Any, Field(alias="foo\\bar")]

