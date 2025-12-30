from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref8Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    ref: Annotated[str | None, Field(alias="$ref", default=None)]

