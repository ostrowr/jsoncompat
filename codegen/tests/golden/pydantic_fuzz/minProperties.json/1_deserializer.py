from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

class Minproperties1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")

    @model_validator(mode="after")
    def _check_properties(self):
        keys = set(self.model_fields_set)
        extra = getattr(self, "__pydantic_extra__", None)
        if extra:
            keys.update(extra.keys())
        if len(keys) < 1:
            raise ValueError("expected at least 1 properties")
        return self

