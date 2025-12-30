from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import PydanticUndefined

class ModelSerializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    length: str = Field(default_factory=lambda: PydanticUndefined)

class Properties5Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    proto: float = Field(alias="__proto__", default_factory=lambda: PydanticUndefined)
    constructor: float = Field(default_factory=lambda: PydanticUndefined)
    to_string: ModelSerializer = Field(alias="toString", default_factory=lambda: PydanticUndefined)

    @model_validator(mode="wrap")
    def _allow_non_objects(cls, value, handler):
        if not isinstance(value, dict):
            inst = cls.model_construct()
            setattr(inst, "_jsonschema_codegen_skip_object_checks", True)
            return inst
        return handler(value)

