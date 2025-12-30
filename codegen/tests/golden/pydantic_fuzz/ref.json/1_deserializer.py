from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import PydanticUndefined

class Ref1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: int = Field(default_factory=lambda: PydanticUndefined)
    foo: int = Field(default_factory=lambda: PydanticUndefined)

