from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFoo(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"minProperties\":0,\"properties\":{\"bar\":{\"minLength\":0,\"type\":\"string\"}},\"type\":\"object\",\"unevaluatedProperties\":false}"
    bar: (str | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("bar", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"anyOf\":[{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{\"foo\":{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{\"faz\":{\"minLength\":0,\"type\":\"string\"}},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}],\"minProperties\":0,\"properties\":{\"foo\":{\"minProperties\":0,\"properties\":{\"bar\":{\"minLength\":0,\"type\":\"string\"}},\"type\":\"object\",\"unevaluatedProperties\":false}},\"type\":\"object\"}"
    foo: (GeneratedSchemaFoo | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchemaFoo.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("bar", "bar", (str | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("foo", "foo", (GeneratedSchemaFoo | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
