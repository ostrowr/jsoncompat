from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import PydanticUndefined

class ModelDeserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    length: str = Field(default_factory=lambda: PydanticUndefined)

class Properties5Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    proto: float = Field(alias="__proto__", default_factory=lambda: PydanticUndefined)
    constructor: float = Field(default_factory=lambda: PydanticUndefined)
    to_string: ModelDeserializer = Field(alias="toString", default_factory=lambda: PydanticUndefined)

