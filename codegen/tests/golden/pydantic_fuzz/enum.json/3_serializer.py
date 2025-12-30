from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Enum3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[Literal["bar"], Field()]
    foo: Annotated[Literal["foo"] | None, Field(default=None)]

