from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field
from pydantic_core import PydanticUndefined

class Default2Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    alpha: Annotated[float, Field(le=3.0)] = Field(default_factory=lambda: PydanticUndefined)

