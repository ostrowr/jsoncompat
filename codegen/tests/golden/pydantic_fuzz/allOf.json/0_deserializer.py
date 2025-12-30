from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Allof0Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int, Field()]
    foo: Annotated[str, Field()]

