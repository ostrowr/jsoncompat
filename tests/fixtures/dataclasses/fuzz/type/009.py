from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "properties": {},
  "type": "object"
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = dc.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "array",
    "object"
  ]
}"""
    root: (GeneratedSchemaBranch1 | list[GeneratedSchemaItem]) = dc.jsoncompat_root_field()

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch1.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchema.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch1 | list[GeneratedSchemaItem])

JSONCOMPAT_MODEL = GeneratedSchema
