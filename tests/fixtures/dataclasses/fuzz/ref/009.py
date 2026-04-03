from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBool(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "true"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"bool\":true},\"$ref\":\"#/$defs/bool\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\"}"
    root: GeneratedSchemaBool = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBool.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaBool

JSONCOMPAT_MODEL = GeneratedSchema
