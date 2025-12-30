from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref12Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo_bar: Annotated[float | None, Field(alias="foo\"bar", default=None)]

