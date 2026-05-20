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
    next: GeneratedSchemaB = dc.field("next")

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
    root: (GeneratedSchemaABranch1 | None) = dc.root_field()

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
    next: GeneratedSchemaA = dc.field("next")

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
    root: (GeneratedSchemaBBranch1 | None) = dc.root_field()

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
    root: GeneratedSchemaA = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
