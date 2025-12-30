from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Unevaluatedproperties15Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")

