from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBar(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"bar\":{\"prefixItems\":[true,{\"type\":\"string\"}]}},\"prefixItems\":[true,{\"type\":\"string\"}]}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"bar\":{\"prefixItems\":[true,{\"type\":\"string\"}]}},\"$ref\":\"#/$defs/bar\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"prefixItems\":[{\"type\":\"string\"}],\"unevaluatedItems\":false}"
    root: GeneratedSchemaBar = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBar.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaBar

JSONCOMPAT_MODEL = GeneratedSchema
