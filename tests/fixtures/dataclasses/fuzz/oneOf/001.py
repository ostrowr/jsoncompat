from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minLength": 0,
  "minProperties": 0,
  "properties": {},
  "type": "object"
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch22(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minLength": 0,
  "minProperties": 0,
  "properties": {},
  "type": "object"
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem2(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "minLength": 2
    },
    {
      "maxLength": 4
    }
  ],
  "type": "string"
}"""
    root: (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch22 | float | list[GeneratedSchemaItem2] | str | None)) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch22.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaItem2.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch22 | float | list[GeneratedSchemaItem2] | str | None))

JSONCOMPAT_MODEL = GeneratedSchema
