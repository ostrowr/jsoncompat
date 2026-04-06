from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileV1(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "v1": {
      "properties": {
        "age": {
          "minimum": 0,
          "type": "integer"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "age"
      ],
      "type": "object",
      "x-jsoncompat": {
        "kind": "declaration",
        "name": "ExamplesStampUserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "examples/stamp/user-profile",
        "version": 1
      }
    },
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
    "name": {
      "minLength": 1,
      "type": "string"
    }
  },
  "required": [
    "name",
    "age"
  ],
  "type": "object",
  "x-jsoncompat": {
    "kind": "declaration",
    "name": "ExamplesStampUserProfileV1",
    "schema_ref": "#/$defs/v1",
    "stable_id": "examples/stamp/user-profile",
    "version": 1
  }
}"""
    age: int = dc.field("age")
    name: str = dc.field("name")
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileV2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "v1": {
      "properties": {
        "age": {
          "minimum": 0,
          "type": "integer"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "age"
      ],
      "type": "object",
      "x-jsoncompat": {
        "kind": "declaration",
        "name": "ExamplesStampUserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "examples/stamp/user-profile",
        "version": 1
      }
    },
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
class ExamplesStampUserProfileV2Reader(dc.ReaderDataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "v1": {
      "properties": {
        "age": {
          "minimum": 0,
          "type": "integer"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "age"
      ],
      "type": "object",
      "x-jsoncompat": {
        "kind": "declaration",
        "name": "ExamplesStampUserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "examples/stamp/user-profile",
        "version": 1
      }
    },
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
  "type": "object",
  "x-jsoncompat": {
    "kind": "reader_variant",
    "name": "ExamplesStampUserProfileV2Reader",
    "payload_ref": "#/$defs/v2",
    "stable_id": "examples/stamp/user-profile",
    "version": 2
  }
}"""
    version: typing.Literal[2] = dc.field("version")
    data: ExamplesStampUserProfileV2 = dc.field("data")

@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileV1Reader(dc.ReaderDataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "v1": {
      "properties": {
        "age": {
          "minimum": 0,
          "type": "integer"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "age"
      ],
      "type": "object",
      "x-jsoncompat": {
        "kind": "declaration",
        "name": "ExamplesStampUserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "examples/stamp/user-profile",
        "version": 1
      }
    },
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
  "additionalProperties": false,
  "properties": {
    "data": {
      "$ref": "#/$defs/v1"
    },
    "version": {
      "const": 1
    }
  },
  "required": [
    "version",
    "data"
  ],
  "type": "object",
  "x-jsoncompat": {
    "kind": "reader_variant",
    "name": "ExamplesStampUserProfileV1Reader",
    "payload_ref": "#/$defs/v1",
    "stable_id": "examples/stamp/user-profile",
    "version": 1
  }
}"""
    version: typing.Literal[1] = dc.field("version")
    data: ExamplesStampUserProfileV1 = dc.field("data")


@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileReader(dc.ReaderDataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "v1": {
      "properties": {
        "age": {
          "minimum": 0,
          "type": "integer"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "age"
      ],
      "type": "object",
      "x-jsoncompat": {
        "kind": "declaration",
        "name": "ExamplesStampUserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "examples/stamp/user-profile",
        "version": 1
      }
    },
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
  "oneOf": [
    {
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
      "type": "object",
      "x-jsoncompat": {
        "kind": "reader_variant",
        "name": "ExamplesStampUserProfileV2Reader",
        "payload_ref": "#/$defs/v2",
        "stable_id": "examples/stamp/user-profile",
        "version": 2
      }
    },
    {
      "additionalProperties": false,
      "properties": {
        "data": {
          "$ref": "#/$defs/v1"
        },
        "version": {
          "const": 1
        }
      },
      "required": [
        "version",
        "data"
      ],
      "type": "object",
      "x-jsoncompat": {
        "kind": "reader_variant",
        "name": "ExamplesStampUserProfileV1Reader",
        "payload_ref": "#/$defs/v1",
        "stable_id": "examples/stamp/user-profile",
        "version": 1
      }
    }
  ],
  "title": "examples/stamp/user-profile reader",
  "x-jsoncompat": {
    "kind": "reader",
    "name": "ExamplesStampUserProfileReader",
    "stable_id": "examples/stamp/user-profile"
  }
}"""
    root: (ExamplesStampUserProfileV1Reader | ExamplesStampUserProfileV2Reader) = dc.root_field()

JSONCOMPAT_MODEL = ExamplesStampUserProfileReader
