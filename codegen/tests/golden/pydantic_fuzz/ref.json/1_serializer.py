from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref1Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int | None, Field(default=None)]
    foo: Annotated[int | None, Field(default=None)]

