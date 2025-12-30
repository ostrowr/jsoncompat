from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import PydanticUndefined

class Ref3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    percent: int = Field(default_factory=lambda: PydanticUndefined)
    slash: int = Field(default_factory=lambda: PydanticUndefined)
    tilde: int = Field(default_factory=lambda: PydanticUndefined)

