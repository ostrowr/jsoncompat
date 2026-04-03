from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaStart(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$comment\":\"this is the landing spot from $ref\",\"$defs\":{\"start\":{\"$comment\":\"this is the landing spot from $ref\",\"$dynamicRef\":\"inner_scope#thingy\",\"$id\":\"start\"},\"thingy\":{\"$comment\":\"this is the first stop for the $dynamicRef\",\"$dynamicAnchor\":\"thingy\",\"$id\":\"inner_scope\",\"type\":\"string\"}},\"$dynamicRef\":\"inner_scope#thingy\",\"$id\":\"start\"}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaThingy(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$comment\":\"this is the first stop for the $dynamicRef\",\"$defs\":{\"start\":{\"$comment\":\"this is the landing spot from $ref\",\"$dynamicRef\":\"inner_scope#thingy\",\"$id\":\"start\"},\"thingy\":{\"$comment\":\"this is the first stop for the $dynamicRef\",\"$dynamicAnchor\":\"thingy\",\"$id\":\"inner_scope\",\"type\":\"string\"}},\"$dynamicAnchor\":\"thingy\",\"$id\":\"inner_scope\",\"type\":\"string\"}"
    root: str = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"start\":{\"$comment\":\"this is the landing spot from $ref\",\"$dynamicRef\":\"inner_scope#thingy\",\"$id\":\"start\"},\"thingy\":{\"$comment\":\"this is the first stop for the $dynamicRef\",\"$dynamicAnchor\":\"thingy\",\"$id\":\"inner_scope\",\"type\":\"string\"}},\"$id\":\"https://test.json-schema.org/dynamic-ref-leaving-dynamic-scope/main\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"if\":{\"$defs\":{\"thingy\":{\"$comment\":\"this is first_scope#thingy\",\"$dynamicAnchor\":\"thingy\",\"type\":\"number\"}},\"$id\":\"first_scope\"},\"then\":{\"$defs\":{\"thingy\":{\"$comment\":\"this is second_scope#thingy, the final destination of the $dynamicRef\",\"$dynamicAnchor\":\"thingy\",\"type\":\"null\"}},\"$id\":\"second_scope\",\"$ref\":\"start\"}}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaStart.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaThingy.__jsoncompat_root_annotation__ = str

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
