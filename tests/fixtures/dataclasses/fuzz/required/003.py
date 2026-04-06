from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar2(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar3(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar4(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar5(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar6(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 6,
  "properties": {
    "foo\\tbar": true,
    "foo\\nbar": true,
    "foo\\fbar": true,
    "foo\\rbar": true,
    "foo\\"bar": true,
    "foo\\\\bar": true
  },
  "required": [
    "foo\\tbar",
    "foo\\nbar",
    "foo\\fbar",
    "foo\\rbar",
    "foo\\"bar",
    "foo\\\\bar"
  ],
  "type": "object"
}"""
    foo_bar: GeneratedSchemaBranch2FooBar = jsoncompat_dataclasses.jsoncompat_field("foo\tbar")
    foo_bar2: GeneratedSchemaBranch2FooBar2 = jsoncompat_dataclasses.jsoncompat_field("foo\nbar")
    foo_bar3: GeneratedSchemaBranch2FooBar3 = jsoncompat_dataclasses.jsoncompat_field("foo\fbar")
    foo_bar4: GeneratedSchemaBranch2FooBar4 = jsoncompat_dataclasses.jsoncompat_field("foo\rbar")
    foo_bar5: GeneratedSchemaBranch2FooBar5 = jsoncompat_dataclasses.jsoncompat_field("foo\"bar")
    foo_bar6: GeneratedSchemaBranch2FooBar6 = jsoncompat_dataclasses.jsoncompat_field("foo\\bar")
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
    "foo\\nbar",
    "foo\\"bar",
    "foo\\\\bar",
    "foo\\rbar",
    "foo\\tbar",
    "foo\\fbar"
  ]
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | None | float | list[GeneratedSchemaItem] | str) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch2FooBar.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2FooBar2.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2FooBar3.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2FooBar4.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2FooBar5.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2FooBar6.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar", "foo\tbar", GeneratedSchemaBranch2FooBar),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar2", "foo\nbar", GeneratedSchemaBranch2FooBar2),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar3", "foo\fbar", GeneratedSchemaBranch2FooBar3),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar4", "foo\rbar", GeneratedSchemaBranch2FooBar4),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar5", "foo\"bar", GeneratedSchemaBranch2FooBar5),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar6", "foo\\bar", GeneratedSchemaBranch2FooBar6),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | None | float | list[GeneratedSchemaItem] | str)

JSONCOMPAT_MODEL = GeneratedSchema
