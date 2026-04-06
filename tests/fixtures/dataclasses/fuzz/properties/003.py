from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"minProperties\":0,\"properties\":{\"foo\\tbar\":{\"type\":\"number\"},\"foo\\nbar\":{\"type\":\"number\"},\"foo\\fbar\":{\"type\":\"number\"},\"foo\\rbar\":{\"type\":\"number\"},\"foo\\\"bar\":{\"type\":\"number\"},\"foo\\\\bar\":{\"type\":\"number\"}},\"type\":\"object\"}"
    foo_bar: (float | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("foo\tbar", omittable=True)
    foo_bar2: (float | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("foo\nbar", omittable=True)
    foo_bar3: (float | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("foo\fbar", omittable=True)
    foo_bar4: (float | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("foo\rbar", omittable=True)
    foo_bar5: (float | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("foo\"bar", omittable=True)
    foo_bar6: (float | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("foo\\bar", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "true"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{\"foo\\tbar\":{\"type\":\"number\"},\"foo\\nbar\":{\"type\":\"number\"},\"foo\\fbar\":{\"type\":\"number\"},\"foo\\rbar\":{\"type\":\"number\"},\"foo\\\"bar\":{\"type\":\"number\"},\"foo\\\\bar\":{\"type\":\"number\"}},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}"
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | typing.Literal[None]) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar", "foo\tbar", (float | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar2", "foo\nbar", (float | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar3", "foo\fbar", (float | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar4", "foo\rbar", (float | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar5", "foo\"bar", (float | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar6", "foo\\bar", (float | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | typing.Literal[None])

JSONCOMPAT_MODEL = GeneratedSchema
