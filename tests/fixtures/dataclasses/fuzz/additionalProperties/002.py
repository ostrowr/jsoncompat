from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Bar(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Foo(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[(typing.Literal[False] | typing.Literal[True])]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "enum": [
      false,
      true
    ]
  },
  "minProperties": 0,
  "properties": {
    "bar": true,
    "foo": true
  },
  "type": "object"
}"""
    bar: dc.Omittable[GeneratedSchemaBranch2Bar] = dc.field("bar", omittable=True)
    foo: dc.Omittable[GeneratedSchemaBranch2Foo] = dc.field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, (typing.Literal[False] | typing.Literal[True])] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "boolean"
  },
  "properties": {
    "bar": {},
    "foo": {}
  }
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) = dc.root_field()

GeneratedSchemaBranch2Bar.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2Foo.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2.__jsoncompat_object_spec__ = dc.object_spec(
    dc.field_spec("bar", "bar", (GeneratedSchemaBranch2Bar | dc.JsoncompatMissingType), omittable=True),
    dc.field_spec("foo", "foo", (GeneratedSchemaBranch2Foo | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, (typing.Literal[False] | typing.Literal[True])],
)

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None)

JSONCOMPAT_MODEL = GeneratedSchema
