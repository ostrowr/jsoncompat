from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaABranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "A": {
      "unevaluatedProperties": false
    }
  },
  "minProperties": 0,
  "properties": {},
  "type": "object",
  "unevaluatedProperties": false
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaAItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaA(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "A": {
      "unevaluatedProperties": false
    }
  },
  "unevaluatedProperties": false
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaABranch2 | float | list[GeneratedSchemaAItem] | str | None) = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "A": {
      "unevaluatedProperties": false
    }
  },
  "$ref": "#/$defs/A",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "prop1": {
      "type": "string"
    }
  }
}"""
    prop1: dc.Omittable[str] = dc.field("prop1", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

GeneratedSchemaABranch2.__jsoncompat_object_spec__ = dc.object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaAItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaA.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaABranch2 | float | list[GeneratedSchemaAItem] | str | None)

GeneratedSchema.__jsoncompat_object_spec__ = dc.object_spec(
    dc.field_spec("prop1", "prop1", (str | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
