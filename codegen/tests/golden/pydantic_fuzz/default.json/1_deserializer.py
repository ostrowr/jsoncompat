from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Default1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[str, Field(min_length=4)] = Field(default="bad")

