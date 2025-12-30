from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Required4Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    proto: Any = Field(alias="__proto__")
    constructor: Any
    to_string: Any = Field(alias="toString")

