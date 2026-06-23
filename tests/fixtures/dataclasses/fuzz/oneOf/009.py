from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Bar(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2Baz(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 1,
  "properties": {
    "bar": true,
    "baz": true
  },
  "required": [
    "bar"
  ],
  "type": "object"
}"""
    bar: GeneratedSchemaBranch2Bar = dc.field("bar")
    baz: dc.Omittable[GeneratedSchemaBranch2Baz] = dc.field("baz", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch22Foo(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch22(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 1,
  "properties": {
    "foo": true
  },
  "required": [
    "foo"
  ],
  "type": "object"
}"""
    foo: GeneratedSchemaBranch22Foo = dc.field("foo")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "properties": {
        "bar": true,
        "baz": true
      },
      "required": [
        "bar"
      ]
    },
    {
      "properties": {
        "foo": true
      },
      "required": [
        "foo"
      ]
    }
  ]
}"""
    root: (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[GeneratedSchemaItem] | float | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch22 | collections.abc.Sequence[GeneratedSchemaItem2] | float | str | None)) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchemaBranch2Bar, "root", typing.Any),
    (GeneratedSchemaBranch2Baz, "root", typing.Any),
    (
        GeneratedSchemaBranch2,
        "object",
        (
            ("bar", "bar", GeneratedSchemaBranch2Bar, False),
            ("baz", "baz", GeneratedSchemaBranch2Baz, True),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaItem, "root", typing.Any),
    (GeneratedSchemaBranch22Foo, "root", typing.Any),
    (
        GeneratedSchemaBranch22,
        "object",
        (
            ("foo", "foo", GeneratedSchemaBranch22Foo, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaItem2, "root", typing.Any),
    (GeneratedSchema, "root", (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[GeneratedSchemaItem] | float | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch22 | collections.abc.Sequence[GeneratedSchemaItem2] | float | str | None))),
))
