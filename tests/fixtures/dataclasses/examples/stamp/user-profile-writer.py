from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileV2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "v2": {
      "properties": {
        "age": {
          "minimum": 0,
          "type": "integer"
        },
        "interests": {
          "type": "integer"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "age",
        "interests"
      ],
      "type": "object",
      "x-jsoncompat": {
        "kind": "declaration",
        "name": "ExamplesStampUserProfileV2",
        "schema_ref": "#/$defs/v2",
        "stable_id": "examples/stamp/user-profile",
        "version": 2
      }
    }
  },
  "properties": {
    "age": {
      "minimum": 0,
      "type": "integer"
    },
    "interests": {
      "type": "integer"
    },
    "name": {
      "minLength": 1,
      "type": "string"
    }
  },
  "required": [
    "name",
    "age",
    "interests"
  ],
  "type": "object",
  "x-jsoncompat": {
    "kind": "declaration",
    "name": "ExamplesStampUserProfileV2",
    "schema_ref": "#/$defs/v2",
    "stable_id": "examples/stamp/user-profile",
    "version": 2
  }
}"""
    age: int = dc.field("age")
    interests: int = dc.field("interests")
    name: str = dc.field("name")
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileWriter(dc.WriterDataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "v2": {
      "properties": {
        "age": {
          "minimum": 0,
          "type": "integer"
        },
        "interests": {
          "type": "integer"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "age",
        "interests"
      ],
      "type": "object",
      "x-jsoncompat": {
        "kind": "declaration",
        "name": "ExamplesStampUserProfileV2",
        "schema_ref": "#/$defs/v2",
        "stable_id": "examples/stamp/user-profile",
        "version": 2
      }
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "data": {
      "$ref": "#/$defs/v2"
    },
    "version": {
      "const": 2
    }
  },
  "required": [
    "version",
    "data"
  ],
  "title": "examples/stamp/user-profile writer v2",
  "type": "object",
  "x-jsoncompat": {
    "kind": "writer",
    "name": "ExamplesStampUserProfileWriter",
    "payload_ref": "#/$defs/v2",
    "stable_id": "examples/stamp/user-profile",
    "version": 2
  }
}"""
    version: typing.Literal[2] = dc.field("version")
    data: ExamplesStampUserProfileV2 = dc.field("data")

JSONCOMPAT_MODEL = ExamplesStampUserProfileWriter
