from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaDirectoryMetadata(dc.DataclassAdditionalModel[str]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "directory": {
      "additionalProperties": false,
      "properties": {
        "children": {
          "items": {
            "$ref": "#/$defs/node"
          },
          "type": "array"
        },
        "kind": {
          "const": "directory"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "owner": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "kind",
        "name",
        "owner",
        "children",
        "metadata"
      ],
      "type": "object"
    },
    "file": {
      "additionalProperties": false,
      "properties": {
        "checksum": {
          "type": [
            "string",
            "null"
          ]
        },
        "kind": {
          "const": "file"
        },
        "labels": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "size": {
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "kind",
        "name",
        "size",
        "labels",
        "metadata"
      ],
      "type": "object"
    },
    "node": {
      "oneOf": [
        {
          "$ref": "#/$defs/file"
        },
        {
          "$ref": "#/$defs/directory"
        }
      ]
    }
  },
  "additionalProperties": {
    "type": "string"
  },
  "type": "object"
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, str] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaDirectory(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "directory": {
      "additionalProperties": false,
      "properties": {
        "children": {
          "items": {
            "$ref": "#/$defs/node"
          },
          "type": "array"
        },
        "kind": {
          "const": "directory"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "owner": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "kind",
        "name",
        "owner",
        "children",
        "metadata"
      ],
      "type": "object"
    },
    "file": {
      "additionalProperties": false,
      "properties": {
        "checksum": {
          "type": [
            "string",
            "null"
          ]
        },
        "kind": {
          "const": "file"
        },
        "labels": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "size": {
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "kind",
        "name",
        "size",
        "labels",
        "metadata"
      ],
      "type": "object"
    },
    "node": {
      "oneOf": [
        {
          "$ref": "#/$defs/file"
        },
        {
          "$ref": "#/$defs/directory"
        }
      ]
    }
  },
  "additionalProperties": false,
  "properties": {
    "children": {
      "items": {
        "$ref": "#/$defs/node"
      },
      "type": "array"
    },
    "kind": {
      "const": "directory"
    },
    "metadata": {
      "additionalProperties": {
        "type": "string"
      },
      "type": "object"
    },
    "name": {
      "minLength": 1,
      "type": "string"
    },
    "owner": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "kind",
    "name",
    "owner",
    "children",
    "metadata"
  ],
  "type": "object"
}"""
    children: collections.abc.Sequence[GeneratedSchemaNode] = dc.field("children")
    kind: typing.Literal["directory"] = dc.field("kind")
    metadata: GeneratedSchemaDirectoryMetadata = dc.field("metadata")
    name: str = dc.field("name")
    owner: (str | None) = dc.field("owner")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFileMetadata(dc.DataclassAdditionalModel[str]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "directory": {
      "additionalProperties": false,
      "properties": {
        "children": {
          "items": {
            "$ref": "#/$defs/node"
          },
          "type": "array"
        },
        "kind": {
          "const": "directory"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "owner": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "kind",
        "name",
        "owner",
        "children",
        "metadata"
      ],
      "type": "object"
    },
    "file": {
      "additionalProperties": false,
      "properties": {
        "checksum": {
          "type": [
            "string",
            "null"
          ]
        },
        "kind": {
          "const": "file"
        },
        "labels": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "size": {
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "kind",
        "name",
        "size",
        "labels",
        "metadata"
      ],
      "type": "object"
    },
    "node": {
      "oneOf": [
        {
          "$ref": "#/$defs/file"
        },
        {
          "$ref": "#/$defs/directory"
        }
      ]
    }
  },
  "additionalProperties": {
    "type": "string"
  },
  "type": "object"
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, str] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFile(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "directory": {
      "additionalProperties": false,
      "properties": {
        "children": {
          "items": {
            "$ref": "#/$defs/node"
          },
          "type": "array"
        },
        "kind": {
          "const": "directory"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "owner": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "kind",
        "name",
        "owner",
        "children",
        "metadata"
      ],
      "type": "object"
    },
    "file": {
      "additionalProperties": false,
      "properties": {
        "checksum": {
          "type": [
            "string",
            "null"
          ]
        },
        "kind": {
          "const": "file"
        },
        "labels": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "size": {
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "kind",
        "name",
        "size",
        "labels",
        "metadata"
      ],
      "type": "object"
    },
    "node": {
      "oneOf": [
        {
          "$ref": "#/$defs/file"
        },
        {
          "$ref": "#/$defs/directory"
        }
      ]
    }
  },
  "additionalProperties": false,
  "properties": {
    "checksum": {
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "const": "file"
    },
    "labels": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "metadata": {
      "additionalProperties": {
        "type": "string"
      },
      "type": "object"
    },
    "name": {
      "minLength": 1,
      "type": "string"
    },
    "size": {
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "kind",
    "name",
    "size",
    "labels",
    "metadata"
  ],
  "type": "object"
}"""
    checksum: dc.Omittable[str | None] = dc.field("checksum", omittable=True)
    kind: typing.Literal["file"] = dc.field("kind")
    labels: collections.abc.Sequence[str] = dc.field("labels")
    metadata: GeneratedSchemaFileMetadata = dc.field("metadata")
    name: str = dc.field("name")
    size: int = dc.field("size")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaNode(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "directory": {
      "additionalProperties": false,
      "properties": {
        "children": {
          "items": {
            "$ref": "#/$defs/node"
          },
          "type": "array"
        },
        "kind": {
          "const": "directory"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "owner": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "kind",
        "name",
        "owner",
        "children",
        "metadata"
      ],
      "type": "object"
    },
    "file": {
      "additionalProperties": false,
      "properties": {
        "checksum": {
          "type": [
            "string",
            "null"
          ]
        },
        "kind": {
          "const": "file"
        },
        "labels": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "size": {
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "kind",
        "name",
        "size",
        "labels",
        "metadata"
      ],
      "type": "object"
    },
    "node": {
      "oneOf": [
        {
          "$ref": "#/$defs/file"
        },
        {
          "$ref": "#/$defs/directory"
        }
      ]
    }
  },
  "oneOf": [
    {
      "$ref": "#/$defs/file"
    },
    {
      "$ref": "#/$defs/directory"
    }
  ]
}"""
    root: (GeneratedSchemaDirectory | GeneratedSchemaFile) = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "directory": {
      "additionalProperties": false,
      "properties": {
        "children": {
          "items": {
            "$ref": "#/$defs/node"
          },
          "type": "array"
        },
        "kind": {
          "const": "directory"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "owner": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "kind",
        "name",
        "owner",
        "children",
        "metadata"
      ],
      "type": "object"
    },
    "file": {
      "additionalProperties": false,
      "properties": {
        "checksum": {
          "type": [
            "string",
            "null"
          ]
        },
        "kind": {
          "const": "file"
        },
        "labels": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "metadata": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        },
        "size": {
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "kind",
        "name",
        "size",
        "labels",
        "metadata"
      ],
      "type": "object"
    },
    "node": {
      "oneOf": [
        {
          "$ref": "#/$defs/file"
        },
        {
          "$ref": "#/$defs/directory"
        }
      ]
    }
  },
  "$ref": "#/$defs/node"
}"""
    root: GeneratedSchemaNode = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
