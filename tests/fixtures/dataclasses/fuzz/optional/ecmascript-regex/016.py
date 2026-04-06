from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "patternProperties": {
    "\\\\wcole": true
  },
  "type": "object"
}"""
    pass

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
)

JSONCOMPAT_MODEL = GeneratedSchema
