from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaA(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"a\":{\"type\":\"integer\"},\"b\":{\"$ref\":\"#/$defs/a\"},\"c\":{\"$ref\":\"#/$defs/b\"}},\"type\":\"integer\"}"
    root: int = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaB(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"a\":{\"type\":\"integer\"},\"b\":{\"$ref\":\"#/$defs/a\"},\"c\":{\"$ref\":\"#/$defs/b\"}},\"$ref\":\"#/$defs/a\"}"
    root: GeneratedSchemaA = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaC(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"a\":{\"type\":\"integer\"},\"b\":{\"$ref\":\"#/$defs/a\"},\"c\":{\"$ref\":\"#/$defs/b\"}},\"$ref\":\"#/$defs/b\"}"
    root: GeneratedSchemaB = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"a\":{\"type\":\"integer\"},\"b\":{\"$ref\":\"#/$defs/a\"},\"c\":{\"$ref\":\"#/$defs/b\"}},\"$ref\":\"#/$defs/c\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\"}"
    root: GeneratedSchemaC = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaA.__jsoncompat_root_annotation__ = int

GeneratedSchemaB.__jsoncompat_root_annotation__ = GeneratedSchemaA

GeneratedSchemaC.__jsoncompat_root_annotation__ = GeneratedSchemaB

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaC

JSONCOMPAT_MODEL = GeneratedSchema
