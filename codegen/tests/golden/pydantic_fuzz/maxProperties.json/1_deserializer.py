from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

class Maxproperties1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")

    @model_validator(mode="wrap")
    def _allow_non_objects(cls, value, handler):
        if not isinstance(value, dict):
            inst = cls.model_construct()
            setattr(inst, "_jsonschema_codegen_skip_object_checks", True)
            return inst
        return handler(value)

    @model_validator(mode="after")
    def _check_properties(self):
        if getattr(self, "_jsonschema_codegen_skip_object_checks", False):
            return self
        count = len(self.model_fields_set)
        extra = getattr(self, "__pydantic_extra__", None)
        if extra:
            count += len(extra)
        if count > 2:
            raise ValueError("expected at most 2 properties")
        return self

