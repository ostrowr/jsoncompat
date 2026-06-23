from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar3(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar4(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar5(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2FooBar6(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[typing.Any]):
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
    foo_bar: GeneratedSchemaBranch2FooBar = dc.field("foo\tbar")
    foo_bar2: GeneratedSchemaBranch2FooBar2 = dc.field("foo\nbar")
    foo_bar3: GeneratedSchemaBranch2FooBar3 = dc.field("foo\fbar")
    foo_bar4: GeneratedSchemaBranch2FooBar4 = dc.field("foo\rbar")
    foo_bar5: GeneratedSchemaBranch2FooBar5 = dc.field("foo\"bar")
    foo_bar6: GeneratedSchemaBranch2FooBar6 = dc.field("foo\\bar")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
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
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[GeneratedSchemaItem] | float | str | None) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchemaBranch2FooBar, "root", typing.Any),
    (GeneratedSchemaBranch2FooBar2, "root", typing.Any),
    (GeneratedSchemaBranch2FooBar3, "root", typing.Any),
    (GeneratedSchemaBranch2FooBar4, "root", typing.Any),
    (GeneratedSchemaBranch2FooBar5, "root", typing.Any),
    (GeneratedSchemaBranch2FooBar6, "root", typing.Any),
    (
        GeneratedSchemaBranch2,
        "object",
        (
            ("foo\tbar", "foo_bar", GeneratedSchemaBranch2FooBar, False),
            ("foo\nbar", "foo_bar2", GeneratedSchemaBranch2FooBar2, False),
            ("foo\fbar", "foo_bar3", GeneratedSchemaBranch2FooBar3, False),
            ("foo\rbar", "foo_bar4", GeneratedSchemaBranch2FooBar4, False),
            ("foo\"bar", "foo_bar5", GeneratedSchemaBranch2FooBar5, False),
            ("foo\\bar", "foo_bar6", GeneratedSchemaBranch2FooBar6, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaItem, "root", typing.Any),
    (GeneratedSchema, "root", ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[GeneratedSchemaItem] | float | str | None)),
))
