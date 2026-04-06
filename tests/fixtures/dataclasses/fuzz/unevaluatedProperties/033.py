from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch2A(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "minProperties": 0,
  "properties": {
    "a": true
  },
  "type": "object"
}"""
    a: jsoncompat_dataclasses.Omittable[GeneratedSchemaOneBranch2A] = jsoncompat_dataclasses.jsoncompat_field("a", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOne(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "properties": {
    "a": true
  }
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch2 | float | list[GeneratedSchemaOneItem] | str | None) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoBranch2X(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoBranch2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "minProperties": 1,
  "properties": {
    "x": true
  },
  "required": [
    "x"
  ],
  "type": "object"
}"""
    x: GeneratedSchemaTwoBranch2X = jsoncompat_dataclasses.jsoncompat_field("x")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoItem(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwo(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "properties": {
    "x": true
  },
  "required": [
    "x"
  ]
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaTwoBranch2 | float | list[GeneratedSchemaTwoItem] | str | None) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "#/$defs/one"
    },
    {
      "properties": {
        "b": true
      }
    },
    {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "y": true
          },
          "required": [
            "y"
          ]
        }
      ]
    }
  ],
  "unevaluatedProperties": false
}"""
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaOneBranch2A.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaOneBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("a", "a", (GeneratedSchemaOneBranch2A | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaOneItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaOne.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch2 | float | list[GeneratedSchemaOneItem] | str | None)

GeneratedSchemaTwoBranch2X.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaTwoBranch2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("x", "x", GeneratedSchemaTwoBranch2X),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaTwoItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaTwo.__jsoncompat_root_annotation__ = ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaTwoBranch2 | float | list[GeneratedSchemaTwoItem] | str | None)

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
