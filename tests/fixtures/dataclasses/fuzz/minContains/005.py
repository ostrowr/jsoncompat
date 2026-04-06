from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"minProperties\":0,\"properties\":{},\"type\":\"object\"}"
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch3(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"not\":true}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"not\":true},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}"
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | GeneratedSchemaBranch3 | float | str | typing.Literal[None]) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBranch3.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | GeneratedSchemaBranch3 | float | str | typing.Literal[None])

JSONCOMPAT_MODEL = GeneratedSchema
