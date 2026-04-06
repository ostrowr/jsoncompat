from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFoo(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$anchor\":\"items\",\"$defs\":{\"foo\":{\"$anchor\":\"items\",\"minLength\":0,\"type\":\"string\"}},\"minLength\":0,\"type\":\"string\"}"
    root: str = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"foo\":{\"$anchor\":\"items\",\"minLength\":0,\"type\":\"string\"}},\"$dynamicRef\":\"#items\"}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"foo\":{\"$anchor\":\"items\",\"minLength\":0,\"type\":\"string\"}},\"$id\":\"https://test.json-schema.org/dynamicRef-anchor-same-schema/root\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"items\":{\"$dynamicRef\":\"#items\"},\"minItems\":0,\"type\":\"array\"}"
    root: list[GeneratedSchemaItem] = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaFoo.__jsoncompat_root_annotation__ = str

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = list[GeneratedSchemaItem]

JSONCOMPAT_MODEL = GeneratedSchema
