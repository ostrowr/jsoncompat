from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMaximum": 9223372036854775807,
  "minimum": 9223372036854775806,
  "type": "integer"
}"""
    root: typing.Literal[9223372036854775806] = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Literal[9223372036854775806]

JSONCOMPAT_MODEL = GeneratedSchema
