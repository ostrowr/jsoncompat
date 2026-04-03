from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"properties\":{\"bar\":{\"type\":\"integer\"}},\"required\":[\"bar\"]}"
    bar: int = jsoncompat_dataclasses.jsoncompat_field("bar")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"properties\":{\"foo\":{\"type\":\"string\"}},\"required\":[\"foo\"]}"
    foo: str = jsoncompat_dataclasses.jsoncompat_field("foo")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"oneOf\":[{\"properties\":{\"bar\":{\"type\":\"integer\"}},\"required\":[\"bar\"]},{\"properties\":{\"foo\":{\"type\":\"string\"}},\"required\":[\"foo\"]}]}"
    root: (GeneratedSchemaBranch0 | GeneratedSchemaBranch1) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch0.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("bar", "bar", int),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBranch1.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("foo", "foo", str),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchema.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch0 | GeneratedSchemaBranch1)

JSONCOMPAT_MODEL = GeneratedSchema
