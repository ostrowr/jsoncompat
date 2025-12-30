from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex1Serializer(SerializerRootModel):
    root: Annotated[str, Field(pattern="^\\t$")]

