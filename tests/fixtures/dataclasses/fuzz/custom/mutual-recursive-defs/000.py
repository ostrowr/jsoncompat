from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaABranch1(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/b"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    },
    "b": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/a"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    }
  },
  "additionalProperties": false,
  "properties": {
    "next": {
      "$ref": "#/$defs/b"
    }
  },
  "required": [
    "next"
  ],
  "type": "object"
}"""
    next: GeneratedSchemaB = dc.jsoncompat_field("next")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaA(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/b"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    },
    "b": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/a"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    }
  },
  "anyOf": [
    {
      "type": "null"
    },
    {
      "additionalProperties": false,
      "properties": {
        "next": {
          "$ref": "#/$defs/b"
        }
      },
      "required": [
        "next"
      ],
      "type": "object"
    }
  ]
}"""
    root: (GeneratedSchemaABranch1 | None) = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBBranch1(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/b"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    },
    "b": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/a"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    }
  },
  "additionalProperties": false,
  "properties": {
    "next": {
      "$ref": "#/$defs/a"
    }
  },
  "required": [
    "next"
  ],
  "type": "object"
}"""
    next: GeneratedSchemaA = dc.jsoncompat_field("next")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaB(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/b"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    },
    "b": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/a"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    }
  },
  "anyOf": [
    {
      "type": "null"
    },
    {
      "additionalProperties": false,
      "properties": {
        "next": {
          "$ref": "#/$defs/a"
        }
      },
      "required": [
        "next"
      ],
      "type": "object"
    }
  ]
}"""
    root: (GeneratedSchemaBBranch1 | None) = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/b"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    },
    "b": {
      "anyOf": [
        {
          "type": "null"
        },
        {
          "additionalProperties": false,
          "properties": {
            "next": {
              "$ref": "#/$defs/a"
            }
          },
          "required": [
            "next"
          ],
          "type": "object"
        }
      ]
    }
  },
  "$ref": "#/$defs/a",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaA = dc.jsoncompat_root_field()

GeneratedSchemaABranch1.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("next", "next", GeneratedSchemaB),
)

GeneratedSchemaA.__jsoncompat_root_annotation__ = (GeneratedSchemaABranch1 | None)

GeneratedSchemaBBranch1.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("next", "next", GeneratedSchemaA),
)

GeneratedSchemaB.__jsoncompat_root_annotation__ = (GeneratedSchemaBBranch1 | None)

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaA

JSONCOMPAT_MODEL = GeneratedSchema
