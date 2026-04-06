from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0Branch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"minLength\":0,\"minProperties\":0,\"properties\":{},\"type\":\"object\"}"
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0Item(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "true"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":2,\"type\":\"string\"},{\"type\":\"number\"}],\"minLength\":0,\"type\":\"string\"}"
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch0Branch2 | float | list[GeneratedSchemaBranch0Item] | str | typing.Literal[None]) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1Branch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"minLength\":0,\"minProperties\":0,\"properties\":{},\"type\":\"object\"}"
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1Item(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "true"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"maxLength\":4,\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}],\"minLength\":0,\"type\":\"string\"}"
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch1Branch2 | float | list[GeneratedSchemaBranch1Item] | str | typing.Literal[None]) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"minLength\":0,\"oneOf\":[{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":2,\"type\":\"string\"},{\"type\":\"number\"}]},{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"maxLength\":4,\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}],\"type\":\"string\"}"
    root: (GeneratedSchemaBranch0 | GeneratedSchemaBranch1) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch0Branch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBranch0Item.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch0.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch0Branch2 | float | list[GeneratedSchemaBranch0Item] | str | typing.Literal[None])

GeneratedSchemaBranch1Branch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBranch1Item.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch1.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch1Branch2 | float | list[GeneratedSchemaBranch1Item] | str | typing.Literal[None])

GeneratedSchema.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch0 | GeneratedSchemaBranch1)

JSONCOMPAT_MODEL = GeneratedSchema
