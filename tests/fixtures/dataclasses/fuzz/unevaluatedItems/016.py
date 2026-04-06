from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBarBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"bar\":{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"prefixItems\":[true,{\"minLength\":0,\"type\":\"string\"}],\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}},\"minProperties\":0,\"properties\":{},\"type\":\"object\"}"
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBar(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"bar\":{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"prefixItems\":[true,{\"minLength\":0,\"type\":\"string\"}],\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}},\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"prefixItems\":[true,{\"minLength\":0,\"type\":\"string\"}],\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}"
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBarBranch2 | float | list[typing.Any] | str | typing.Literal[None]) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"bar\":{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"prefixItems\":[true,{\"minLength\":0,\"type\":\"string\"}],\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}},\"$ref\":\"#/$defs/bar\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"prefixItems\":[{\"minLength\":0,\"type\":\"string\"}],\"unevaluatedItems\":false}"
    root: GeneratedSchemaBar = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBarBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBar.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBarBranch2 | float | list[typing.Any] | str | typing.Literal[None])

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaBar

JSONCOMPAT_MODEL = GeneratedSchema
