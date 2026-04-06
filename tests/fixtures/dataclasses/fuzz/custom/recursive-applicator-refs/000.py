from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTreeBranch1(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "tree": {
      "oneOf": [
        {
          "const": null
        },
        {
          "additionalProperties": false,
          "properties": {
            "left": {
              "$ref": "#/$defs/tree"
            },
            "right": {
              "$ref": "#/$defs/tree"
            }
          },
          "required": [
            "left",
            "right"
          ],
          "type": "object"
        }
      ]
    }
  },
  "additionalProperties": false,
  "minProperties": 2,
  "properties": {
    "left": {
      "$ref": "#/$defs/tree"
    },
    "right": {
      "$ref": "#/$defs/tree"
    }
  },
  "required": [
    "left",
    "right"
  ],
  "type": "object"
}"""
    left: GeneratedSchemaTree = dc.field("left")
    right: GeneratedSchemaTree = dc.field("right")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTree(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "tree": {
      "oneOf": [
        {
          "const": null
        },
        {
          "additionalProperties": false,
          "properties": {
            "left": {
              "$ref": "#/$defs/tree"
            },
            "right": {
              "$ref": "#/$defs/tree"
            }
          },
          "required": [
            "left",
            "right"
          ],
          "type": "object"
        }
      ]
    }
  },
  "oneOf": [
    {
      "const": null
    },
    {
      "additionalProperties": false,
      "properties": {
        "left": {
          "$ref": "#/$defs/tree"
        },
        "right": {
          "$ref": "#/$defs/tree"
        }
      },
      "required": [
        "left",
        "right"
      ],
      "type": "object"
    }
  ]
}"""
    root: (GeneratedSchemaTreeBranch1 | None) = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "tree": {
      "oneOf": [
        {
          "const": null
        },
        {
          "additionalProperties": false,
          "properties": {
            "left": {
              "$ref": "#/$defs/tree"
            },
            "right": {
              "$ref": "#/$defs/tree"
            }
          },
          "required": [
            "left",
            "right"
          ],
          "type": "object"
        }
      ]
    }
  },
  "$ref": "#/$defs/tree",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaTree = dc.root_field()

GeneratedSchemaTreeBranch1.__jsoncompat_object_spec__ = dc.object_spec(
    dc.field_spec("left", "left", GeneratedSchemaTree),
    dc.field_spec("right", "right", GeneratedSchemaTree),
)

GeneratedSchemaTree.__jsoncompat_root_annotation__ = (GeneratedSchemaTreeBranch1 | None)

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaTree

JSONCOMPAT_MODEL = GeneratedSchema
