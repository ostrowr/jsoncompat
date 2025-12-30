from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref11Deserializer(DeserializerBase):
    """tree of nodes"""
    model_config = ConfigDict(extra="allow")
    meta: str
    nodes: list[Any]

