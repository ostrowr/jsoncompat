from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN0(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n4"
        },
        {
          "$ref": "#/$defs/n7"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "not": {
        "$ref": "#/$defs/n3"
      }
    },
    "n3": true,
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "not": {
        "$ref": "#/$defs/n6"
      }
    },
    "n6": true,
    "n7": {
      "$ref": "#/$defs/n8"
    },
    "n8": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/n1"
    },
    {
      "$ref": "#/$defs/n4"
    },
    {
      "$ref": "#/$defs/n7"
    }
  ]
}"""
    root: (GeneratedSchemaN1 | GeneratedSchemaN4 | GeneratedSchemaN7) = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n4"
        },
        {
          "$ref": "#/$defs/n7"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "not": {
        "$ref": "#/$defs/n3"
      }
    },
    "n3": true,
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "not": {
        "$ref": "#/$defs/n6"
      }
    },
    "n6": true,
    "n7": {
      "$ref": "#/$defs/n8"
    },
    "n8": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n2"
}"""
    root: GeneratedSchemaN2 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n4"
        },
        {
          "$ref": "#/$defs/n7"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "not": {
        "$ref": "#/$defs/n3"
      }
    },
    "n3": true,
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "not": {
        "$ref": "#/$defs/n6"
      }
    },
    "n6": true,
    "n7": {
      "$ref": "#/$defs/n8"
    },
    "n8": {
      "minLength": 0,
      "type": "string"
    }
  },
  "not": {
    "$ref": "#/$defs/n3"
  }
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN3(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN4(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n4"
        },
        {
          "$ref": "#/$defs/n7"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "not": {
        "$ref": "#/$defs/n3"
      }
    },
    "n3": true,
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "not": {
        "$ref": "#/$defs/n6"
      }
    },
    "n6": true,
    "n7": {
      "$ref": "#/$defs/n8"
    },
    "n8": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n5"
}"""
    root: GeneratedSchemaN5 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN5(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n4"
        },
        {
          "$ref": "#/$defs/n7"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "not": {
        "$ref": "#/$defs/n3"
      }
    },
    "n3": true,
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "not": {
        "$ref": "#/$defs/n6"
      }
    },
    "n6": true,
    "n7": {
      "$ref": "#/$defs/n8"
    },
    "n8": {
      "minLength": 0,
      "type": "string"
    }
  },
  "not": {
    "$ref": "#/$defs/n6"
  }
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN6(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN7(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n4"
        },
        {
          "$ref": "#/$defs/n7"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "not": {
        "$ref": "#/$defs/n3"
      }
    },
    "n3": true,
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "not": {
        "$ref": "#/$defs/n6"
      }
    },
    "n6": true,
    "n7": {
      "$ref": "#/$defs/n8"
    },
    "n8": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n8"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN8(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n4"
        },
        {
          "$ref": "#/$defs/n7"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "not": {
        "$ref": "#/$defs/n3"
      }
    },
    "n3": true,
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "not": {
        "$ref": "#/$defs/n6"
      }
    },
    "n6": true,
    "n7": {
      "$ref": "#/$defs/n8"
    },
    "n8": {
      "minLength": 0,
      "type": "string"
    }
  },
  "minLength": 0,
  "type": "string"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "id_in_unknown0": {
      "not": {
        "array_of_schemas": [
          {
            "$id": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json",
            "type": "null"
          }
        ]
      }
    },
    "id_in_unknown1": {
      "not": {
        "object_of_schemas": {
          "foo": {
            "$id": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json",
            "type": "integer"
          }
        }
      }
    },
    "real_id_in_schema": {
      "$id": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json",
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/id_in_unknown0"
    },
    {
      "$ref": "#/$defs/id_in_unknown1"
    },
    {
      "$ref": "https://localhost:1234/draft2020-12/unknownKeyword/my_identifier.json"
    }
  ]
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
