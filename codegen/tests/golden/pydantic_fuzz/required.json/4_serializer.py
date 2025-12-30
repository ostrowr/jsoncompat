from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

class Required4Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    proto: Any = Field(alias="__proto__")
    constructor: Any
    to_string: Any = Field(alias="toString")

    @model_validator(mode="wrap")
    def _allow_non_objects(cls, value, handler):
        if not isinstance(value, dict):
            inst = cls.model_construct()
            setattr(inst, "_jsonschema_codegen_skip_object_checks", True)
            return inst
        return handler(value)

