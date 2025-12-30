from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import PydanticUndefined

class Ref0Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="forbid")
    foo: Any = Field(default_factory=lambda: PydanticUndefined)

