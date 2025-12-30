from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import PydanticUndefined

class Unevaluatedproperties16Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo: str = Field(default_factory=lambda: PydanticUndefined)

