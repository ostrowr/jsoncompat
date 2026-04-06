from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Proto(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Constructor(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2ToString(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 3,
  "properties": {
    "__proto__": true,
    "constructor": true,
    "toString": true
  },
  "required": [
    "__proto__",
    "constructor",
    "toString"
  ],
  "type": "object"
}"""
    __proto__: GeneratedSchemaBranch2Proto = jsoncompat_dataclasses.jsoncompat_field("__proto__")
    constructor: GeneratedSchemaBranch2Constructor = jsoncompat_dataclasses.jsoncompat_field("constructor")
    toString: GeneratedSchemaBranch2ToString = jsoncompat_dataclasses.jsoncompat_field("toString")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "required": [
    "__proto__",
    "toString",
    "constructor"
  ]
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch2Proto.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2Constructor.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2ToString.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("__proto__", "__proto__", GeneratedSchemaBranch2Proto),
    jsoncompat_dataclasses.jsoncompat_field_spec("constructor", "constructor", GeneratedSchemaBranch2Constructor),
    jsoncompat_dataclasses.jsoncompat_field_spec("toString", "toString", GeneratedSchemaBranch2ToString),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None)

JSONCOMPAT_MODEL = GeneratedSchema
