from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"item\":{\"items\":false,\"minItems\":0,\"prefixItems\":[{\"$ref\":\"#/$defs/sub-item\"},{\"$ref\":\"#/$defs/sub-item\"}],\"type\":\"array\"},\"sub-item\":{\"minProperties\":1,\"properties\":{\"foo\":true},\"required\":[\"foo\"],\"type\":\"object\"}},\"items\":false,\"minItems\":0,\"prefixItems\":[{\"$ref\":\"#/$defs/sub-item\"},{\"$ref\":\"#/$defs/sub-item\"}],\"type\":\"array\"}"
    root: list[typing.Any] = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaSubItemFoo(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "true"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaSubItem(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"item\":{\"items\":false,\"minItems\":0,\"prefixItems\":[{\"$ref\":\"#/$defs/sub-item\"},{\"$ref\":\"#/$defs/sub-item\"}],\"type\":\"array\"},\"sub-item\":{\"minProperties\":1,\"properties\":{\"foo\":true},\"required\":[\"foo\"],\"type\":\"object\"}},\"minProperties\":1,\"properties\":{\"foo\":true},\"required\":[\"foo\"],\"type\":\"object\"}"
    foo: GeneratedSchemaSubItemFoo = jsoncompat_dataclasses.jsoncompat_field("foo")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"item\":{\"items\":false,\"minItems\":0,\"prefixItems\":[{\"$ref\":\"#/$defs/sub-item\"},{\"$ref\":\"#/$defs/sub-item\"}],\"type\":\"array\"},\"sub-item\":{\"minProperties\":1,\"properties\":{\"foo\":true},\"required\":[\"foo\"],\"type\":\"object\"}},\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"items\":false,\"minItems\":0,\"prefixItems\":[{\"$ref\":\"#/$defs/item\"},{\"$ref\":\"#/$defs/item\"},{\"$ref\":\"#/$defs/item\"}],\"type\":\"array\"}"
    root: list[typing.Any] = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaItem.__jsoncompat_root_annotation__ = list[typing.Any]

GeneratedSchemaSubItemFoo.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaSubItem.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("foo", "foo", GeneratedSchemaSubItemFoo),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchema.__jsoncompat_root_annotation__ = list[typing.Any]

JSONCOMPAT_MODEL = GeneratedSchema
