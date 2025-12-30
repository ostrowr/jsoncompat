from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Items4Deserializer(DeserializerRootModel):
    root: list[list[list[list[float]]]]

