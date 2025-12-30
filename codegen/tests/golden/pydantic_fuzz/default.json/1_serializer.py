from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Default1Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[str | None, Field(min_length=4, default=None)]

