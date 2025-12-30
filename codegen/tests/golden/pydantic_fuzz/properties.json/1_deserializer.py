from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import PydanticUndefined

class Properties1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    __pydantic_extra__: dict[str, int]
    bar: list[Any] = Field(default_factory=lambda: PydanticUndefined)
    foo: Annotated[list[Any], Field(max_length=3)] = Field(default_factory=lambda: PydanticUndefined)

