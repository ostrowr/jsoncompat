from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaStart(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"start\":{\"$dynamicRef\":\"inner_scope#thingy\",\"$id\":\"start\"},\"thingy\":{\"$dynamicAnchor\":\"thingy\",\"$id\":\"inner_scope\",\"minLength\":0,\"type\":\"string\"}},\"$dynamicRef\":\"inner_scope#thingy\",\"$id\":\"start\"}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaThingy(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"start\":{\"$dynamicRef\":\"inner_scope#thingy\",\"$id\":\"start\"},\"thingy\":{\"$dynamicAnchor\":\"thingy\",\"$id\":\"inner_scope\",\"minLength\":0,\"type\":\"string\"}},\"$dynamicAnchor\":\"thingy\",\"$id\":\"inner_scope\",\"minLength\":0,\"type\":\"string\"}"
    root: str = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"start\":{\"$dynamicRef\":\"inner_scope#thingy\",\"$id\":\"start\"},\"thingy\":{\"$dynamicAnchor\":\"thingy\",\"$id\":\"inner_scope\",\"minLength\":0,\"type\":\"string\"}},\"$id\":\"https://test.json-schema.org/dynamic-ref-leaving-dynamic-scope/main\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"if\":{\"$defs\":{\"thingy\":{\"$dynamicAnchor\":\"thingy\",\"type\":\"number\"}},\"$id\":\"first_scope\",\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]},\"then\":{\"$defs\":{\"thingy\":{\"$dynamicAnchor\":\"thingy\",\"enum\":[null]}},\"$id\":\"second_scope\",\"$ref\":\"start\"}}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaStart.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaThingy.__jsoncompat_root_annotation__ = str

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
