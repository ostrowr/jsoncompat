from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Bar(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Foo(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "minProperties": 0,
  "patternProperties": {
    "^v": true
  },
  "properties": {
    "bar": true,
    "foo": true
  },
  "type": "object"
}"""
    bar: jsoncompat_dataclasses.Omittable[GeneratedSchemaBranch2Bar] = jsoncompat_dataclasses.jsoncompat_field("bar", omittable=True)
    foo: jsoncompat_dataclasses.Omittable[GeneratedSchemaBranch2Foo] = jsoncompat_dataclasses.jsoncompat_field("foo", omittable=True)

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "patternProperties": {
    "^v": {}
  },
  "properties": {
    "bar": {},
    "foo": {}
  }
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch2Bar.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2Foo.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("bar", "bar", (GeneratedSchemaBranch2Bar | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo", "foo", (GeneratedSchemaBranch2Foo | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None)

JSONCOMPAT_MODEL = GeneratedSchema
