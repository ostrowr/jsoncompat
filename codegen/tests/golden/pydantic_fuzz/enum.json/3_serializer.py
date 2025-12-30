from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import PydanticUndefined

class Enum3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Literal["bar"]
    foo: Literal["foo"] = Field(default_factory=lambda: PydanticUndefined)

