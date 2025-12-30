from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Anyof1Serializer(SerializerRootModel):
    root: Annotated[str, Field(max_length=2)] | Annotated[str, Field(min_length=4)]

