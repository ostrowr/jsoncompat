from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "properties": {
    "foo\\tbar": {
      "type": "number"
    },
    "foo\\nbar": {
      "type": "number"
    },
    "foo\\fbar": {
      "type": "number"
    },
    "foo\\rbar": {
      "type": "number"
    },
    "foo\\"bar": {
      "type": "number"
    },
    "foo\\\\bar": {
      "type": "number"
    }
  },
  "type": "object"
}"""
    foo_bar: dc.Omittable[float] = dc.field("foo\tbar", omittable=True)
    foo_bar2: dc.Omittable[float] = dc.field("foo\nbar", omittable=True)
    foo_bar3: dc.Omittable[float] = dc.field("foo\fbar", omittable=True)
    foo_bar4: dc.Omittable[float] = dc.field("foo\rbar", omittable=True)
    foo_bar5: dc.Omittable[float] = dc.field("foo\"bar", omittable=True)
    foo_bar6: dc.Omittable[float] = dc.field("foo\\bar", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo\\tbar": {
      "type": "number"
    },
    "foo\\nbar": {
      "type": "number"
    },
    "foo\\fbar": {
      "type": "number"
    },
    "foo\\rbar": {
      "type": "number"
    },
    "foo\\"bar": {
      "type": "number"
    },
    "foo\\\\bar": {
      "type": "number"
    }
  }
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
