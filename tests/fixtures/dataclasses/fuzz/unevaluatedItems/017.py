from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBarBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "bar": {
      "prefixItems": [
        true,
        {
          "type": "string"
        }
      ]
    }
  },
  "minProperties": 0,
  "properties": {},
  "type": "object"
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBar(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "bar": {
      "prefixItems": [
        true,
        {
          "type": "string"
        }
      ]
    }
  },
  "prefixItems": [
    true,
    {
      "type": "string"
    }
  ]
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBarBranch2 | float | list[typing.Any] | str | None) = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "bar": {
      "prefixItems": [
        true,
        {
          "type": "string"
        }
      ]
    }
  },
  "$ref": "#/$defs/bar",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "string"
    }
  ],
  "unevaluatedItems": false
}"""
    root: GeneratedSchemaBar = dc.root_field()

GeneratedSchemaBarBranch2.__jsoncompat_object_spec__ = dc.object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBar.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBarBranch2 | float | list[typing.Any] | str | None)

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaBar

JSONCOMPAT_MODEL = GeneratedSchema
