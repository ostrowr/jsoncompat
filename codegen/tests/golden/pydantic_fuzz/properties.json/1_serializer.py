from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import PydanticUndefined

class Properties1Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    __pydantic_extra__: dict[str, int]
    bar: list[Any] = Field(default_factory=lambda: PydanticUndefined)
    foo: Annotated[list[Any], Field(max_length=3)] = Field(default_factory=lambda: PydanticUndefined)

    @model_validator(mode="wrap")
    def _allow_non_objects(cls, value, handler):
        if not isinstance(value, dict):
            inst = cls.model_construct()
            setattr(inst, "_jsonschema_codegen_skip_object_checks", True)
            return inst
        return handler(value)

