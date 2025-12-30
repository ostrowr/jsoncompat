from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Enum4Deserializer(DeserializerRootModel):
    root: Literal["foo\nbar", "foo\rbar"]

