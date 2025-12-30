from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

class Maxproperties2Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")

    @model_validator(mode="after")
    def _check_properties(self):
        count = len(self.model_fields_set)
        extra = getattr(self, "__pydantic_extra__", None)
        if extra:
            count += len(extra)
        if count > 0:
            raise ValueError("expected at most 0 properties")
        return self

