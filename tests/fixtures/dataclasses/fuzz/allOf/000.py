from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"allOf\":[{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":1,\"properties\":{\"bar\":{\"multipleOf\":1,\"type\":\"integer\"}},\"required\":[\"bar\"],\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]},{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":1,\"properties\":{\"foo\":{\"minLength\":0,\"type\":\"string\"}},\"required\":[\"foo\"],\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}]}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
