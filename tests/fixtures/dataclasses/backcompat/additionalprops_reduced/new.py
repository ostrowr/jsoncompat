from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "type": "object"
}"""
    pass

GeneratedSchema.__jsoncompat_object_spec__ = dc.object_spec(
)

JSONCOMPAT_MODEL = GeneratedSchema
