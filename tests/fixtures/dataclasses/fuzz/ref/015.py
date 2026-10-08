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
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "properties": {
        "foo": {
          "$ref": "#/$defs/n1"
        }
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "properties": {
        "bar": {
          "$ref": "#/$defs/n3"
        }
      }
    },
    "n3": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n1",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/n1"
    }
  }
}"""
    root: GeneratedSchemaN1 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "properties": {
        "foo": {
          "$ref": "#/$defs/n1"
        }
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "properties": {
        "bar": {
          "$ref": "#/$defs/n3"
        }
      }
    },
    "n3": {
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
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "properties": {
        "foo": {
          "$ref": "#/$defs/n1"
        }
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "properties": {
        "bar": {
          "$ref": "#/$defs/n3"
        }
      }
    },
    "n3": {
      "minLength": 0,
      "type": "string"
    }
  },
  "properties": {
    "bar": {
      "$ref": "#/$defs/n3"
    }
  }
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN3(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "properties": {
        "foo": {
          "$ref": "#/$defs/n1"
        }
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "properties": {
        "bar": {
          "$ref": "#/$defs/n3"
        }
      }
    },
    "n3": {
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
  "$id": "http://example.com/schema-relative-uri-defs1.json",
  "$ref": "schema-relative-uri-defs2.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$defs": {
        "inner": {
          "properties": {
            "bar": {
              "type": "string"
            }
          }
        }
      },
      "$id": "schema-relative-uri-defs2.json",
      "$ref": "#/$defs/inner"
    }
  }
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
