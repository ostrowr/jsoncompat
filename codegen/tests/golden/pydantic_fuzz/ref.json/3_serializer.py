from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import PydanticUndefined

class Ref3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    percent: int = Field(default_factory=lambda: PydanticUndefined)
    slash: int = Field(default_factory=lambda: PydanticUndefined)
    tilde: int = Field(default_factory=lambda: PydanticUndefined)

    @model_validator(mode="wrap")
    def _allow_non_objects(cls, value, handler):
        if not isinstance(value, dict):
            inst = cls.model_construct()
            setattr(inst, "_jsonschema_codegen_skip_object_checks", True)
            return inst
        return handler(value)

