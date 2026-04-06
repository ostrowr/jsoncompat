from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0Branch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "prefixItems": [
    {
      "enum": [
        "foo"
      ]
    }
  ],
  "properties": {},
  "type": "object",
  "unevaluatedItems": false
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "prefixItems": [
    true,
    {
      "const": "bar"
    }
  ]
}"""
    root: (GeneratedSchemaBranch0Branch2 | float | list[typing.Any] | str | typing.Any) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1Branch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "prefixItems": [
    {
      "enum": [
        "foo"
      ]
    }
  ],
  "properties": {},
  "type": "object",
  "unevaluatedItems": false
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "prefixItems": [
    true,
    true,
    {
      "const": "baz"
    }
  ]
}"""
    root: (GeneratedSchemaBranch1Branch2 | float | list[typing.Any] | str | typing.Any) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "prefixItems": [
        true,
        {
          "const": "bar"
        }
      ]
    },
    {
      "prefixItems": [
        true,
        true,
        {
          "const": "baz"
        }
      ]
    }
  ],
  "prefixItems": [
    {
      "const": "foo"
    }
  ],
  "unevaluatedItems": false
}"""
    root: (GeneratedSchemaBranch0 | GeneratedSchemaBranch1) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch0Branch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBranch0.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch0Branch2 | float | list[typing.Any] | str | typing.Any)

GeneratedSchemaBranch1Branch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaBranch1.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch1Branch2 | float | list[typing.Any] | str | typing.Any)

GeneratedSchema.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch0 | GeneratedSchemaBranch1)

JSONCOMPAT_MODEL = GeneratedSchema
