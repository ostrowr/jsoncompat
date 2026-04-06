from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"allOf\":[{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":1,\"properties\":{\"foo\":{\"minLength\":0,\"type\":\"string\"}},\"required\":[\"foo\"],\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]},{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":1,\"properties\":{\"baz\":{\"enum\":[null]}},\"required\":[\"baz\"],\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}],\"properties\":{\"bar\":{\"multipleOf\":1,\"type\":\"integer\"}},\"required\":[\"bar\"]}"
    bar: int = jsoncompat_dataclasses.jsoncompat_field("bar")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("bar", "bar", int),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
