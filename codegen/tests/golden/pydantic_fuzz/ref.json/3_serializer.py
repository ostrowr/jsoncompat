from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    percent: Annotated[int | None, Field(default=None)]
    slash: Annotated[int | None, Field(default=None)]
    tilde: Annotated[int | None, Field(default=None)]

