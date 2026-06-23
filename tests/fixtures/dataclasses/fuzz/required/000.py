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
class GeneratedSchemaBranch2Foo(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 1,
  "properties": {
    "bar": true,
    "foo": true
  },
  "required": [
    "foo"
  ],
  "type": "object"
}"""
    bar: dc.Omittable[GeneratedSchemaBranch2Bar] = dc.field("bar", omittable=True)
    foo: GeneratedSchemaBranch2Foo = dc.field("foo")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {},
    "foo": {}
  },
  "required": [
    "foo"
  ]
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[GeneratedSchemaItem] | float | str | None) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
