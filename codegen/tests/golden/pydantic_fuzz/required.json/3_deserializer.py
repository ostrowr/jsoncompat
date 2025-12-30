from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

class Required3Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo_bar: Any = Field(alias="foo\tbar")
    foo_bar_2: Any = Field(alias="foo\nbar")
    foo_bar_3: Any = Field(alias="foo\fbar")
    foo_bar_4: Any = Field(alias="foo\rbar")
    foo_bar_5: Any = Field(alias="foo\"bar")
    foo_bar_6: Any = Field(alias="foo\\bar")

    @model_validator(mode="wrap")
    def _allow_non_objects(cls, value, handler):
        if not isinstance(value, dict):
            inst = cls.model_construct()
            setattr(inst, "_jsonschema_codegen_skip_object_checks", True)
            return inst
        return handler(value)

