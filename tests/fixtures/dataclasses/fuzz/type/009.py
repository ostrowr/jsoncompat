from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "true"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"minProperties\":0,\"properties\":{},\"type\":\"object\"}"
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"anyOf\":[{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"}]}"
    root: (GeneratedSchemaBranch1 | list[GeneratedSchemaItem]) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch1.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchema.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch1 | list[GeneratedSchemaItem])

JSONCOMPAT_MODEL = GeneratedSchema
