from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Required4Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    proto: Annotated[Any, Field(alias="__proto__")]
    constructor: Annotated[Any, Field()]
    to_string: Annotated[Any, Field(alias="toString")]

