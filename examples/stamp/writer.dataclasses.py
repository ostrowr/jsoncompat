from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class UserProfileV2(dc.DataclassAdditionalModel[typing.Any]):
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
        "name": "UserProfileV2",
        "schema_ref": "#/$defs/v2",
        "stable_id": "user-profile",
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
    "name": "UserProfileV2",
    "schema_ref": "#/$defs/v2",
    "stable_id": "user-profile",
    "version": 2
  }
}"""
    age: int = dc.field("age")
    interests: int = dc.field("interests")
    name: str = dc.field("name")
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class UserProfileWriter(dc.WriterDataclassModel):
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
        "name": "UserProfileV2",
        "schema_ref": "#/$defs/v2",
        "stable_id": "user-profile",
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
  "title": "user-profile writer v2",
  "type": "object",
  "x-jsoncompat": {
    "kind": "writer",
    "name": "UserProfileWriter",
    "payload_ref": "#/$defs/v2",
    "stable_id": "user-profile",
    "version": 2
  }
}"""
    version: typing.Literal[2] = dc.field("version")
    data: UserProfileV2 = dc.field("data")

UserProfileV2.__jsoncompat_object_spec__ = dc.object_spec(
    dc.field_spec("age", "age", int),
    dc.field_spec("interests", "interests", int),
    dc.field_spec("name", "name", str),
    extra_annotation=dict[str, typing.Any],
)

UserProfileWriter.__jsoncompat_object_spec__ = dc.object_spec(
    dc.field_spec("version", "version", typing.Literal[2]),
    dc.field_spec("data", "data", UserProfileV2),
)

JSONCOMPAT_MODEL = UserProfileWriter
