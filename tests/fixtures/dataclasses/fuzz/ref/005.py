from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaReffed(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"reffed\":{\"type\":\"array\"}},\"type\":\"array\"}"
    root: list[typing.Any] = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"reffed\":{\"type\":\"array\"}},\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"properties\":{\"foo\":{\"$ref\":\"#/$defs/reffed\",\"maxItems\":2}}}"
    foo: (GeneratedSchemaReffed | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchemaReffed.__jsoncompat_root_annotation__ = list[typing.Any]

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("foo", "foo", (GeneratedSchemaReffed | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
