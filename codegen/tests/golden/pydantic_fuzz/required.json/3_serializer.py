from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Required3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo_bar: Any = Field(alias="foo\tbar")
    foo_bar_2: Any = Field(alias="foo\nbar")
    foo_bar_3: Any = Field(alias="foo\fbar")
    foo_bar_4: Any = Field(alias="foo\rbar")
    foo_bar_5: Any = Field(alias="foo\"bar")
    foo_bar_6: Any = Field(alias="foo\\bar")

