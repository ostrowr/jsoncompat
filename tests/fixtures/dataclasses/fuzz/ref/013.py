from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaABranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"A\":{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\",\"unevaluatedProperties\":false},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}},\"minProperties\":0,\"properties\":{},\"type\":\"object\",\"unevaluatedProperties\":false}"
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaAItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "true"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaA(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"A\":{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\",\"unevaluatedProperties\":false},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}},\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\",\"unevaluatedProperties\":false},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}"
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaABranch2 | float | list[GeneratedSchemaAItem] | str | typing.Literal[None]) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"A\":{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\",\"unevaluatedProperties\":false},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}},\"$ref\":\"#/$defs/A\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"properties\":{\"prop1\":{\"minLength\":0,\"type\":\"string\"}}}"
    prop1: (str | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("prop1", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchemaABranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaAItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaA.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaABranch2 | float | list[GeneratedSchemaAItem] | str | typing.Literal[None])

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("prop1", "prop1", (str | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
