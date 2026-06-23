from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class UserProfileV1(dc.DataclassAdditionalModel[typing.Any]):
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
        "name": "UserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "user-profile",
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
    "name": "UserProfileV1",
    "schema_ref": "#/$defs/v1",
    "stable_id": "user-profile",
    "version": 1
  }
}"""
    age: int = dc.field("age")
    name: str = dc.field("name")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class UserProfileV2(dc.DataclassAdditionalModel[typing.Any]):
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
        "name": "UserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "user-profile",
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
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class UserProfileV2Reader(dc.ReaderDataclassModel):
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
        "name": "UserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "user-profile",
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
        "name": "UserProfileV2",
        "schema_ref": "#/$defs/v2",
        "stable_id": "user-profile",
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
    "name": "UserProfileV2Reader",
    "payload_ref": "#/$defs/v2",
    "stable_id": "user-profile",
    "version": 2
  }
}"""
    version: typing.Literal[2] = dc.field("version")
    data: UserProfileV2 = dc.field("data")

@dataclass(frozen=True, slots=True, kw_only=True)
class UserProfileV1Reader(dc.ReaderDataclassModel):
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
        "name": "UserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "user-profile",
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
        "name": "UserProfileV2",
        "schema_ref": "#/$defs/v2",
        "stable_id": "user-profile",
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
    "name": "UserProfileV1Reader",
    "payload_ref": "#/$defs/v1",
    "stable_id": "user-profile",
    "version": 1
  }
}"""
    version: typing.Literal[1] = dc.field("version")
    data: UserProfileV1 = dc.field("data")


@dataclass(frozen=True, slots=True, kw_only=True)
class UserProfileReader(dc.ReaderDataclassRootModel):
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
        "name": "UserProfileV1",
        "schema_ref": "#/$defs/v1",
        "stable_id": "user-profile",
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
        "name": "UserProfileV2",
        "schema_ref": "#/$defs/v2",
        "stable_id": "user-profile",
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
        "name": "UserProfileV2Reader",
        "payload_ref": "#/$defs/v2",
        "stable_id": "user-profile",
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
        "name": "UserProfileV1Reader",
        "payload_ref": "#/$defs/v1",
        "stable_id": "user-profile",
        "version": 1
      }
    }
  ],
  "title": "user-profile reader",
  "x-jsoncompat": {
    "kind": "reader",
    "name": "UserProfileReader",
    "stable_id": "user-profile"
  }
}"""
    root: (UserProfileV1Reader | UserProfileV2Reader) = dc.root_field()

JSONCOMPAT_MODEL = UserProfileReader

dc.bind_generated_models((
    (
        UserProfileV1,
        "object",
        (
            ("age", "age", int, False),
            ("name", "name", str, False),
        ),
        True,
        typing.Any,
    ),
    (
        UserProfileV2,
        "object",
        (
            ("age", "age", int, False),
            ("interests", "interests", int, False),
            ("name", "name", str, False),
        ),
        True,
        typing.Any,
    ),
    (
        UserProfileV2Reader,
        "object",
        (
            ("version", "version", typing.Literal[2], False),
            ("data", "data", UserProfileV2, False),
        ),
        False,
        None,
    ),
    (
        UserProfileV1Reader,
        "object",
        (
            ("version", "version", typing.Literal[1], False),
            ("data", "data", UserProfileV1, False),
        ),
        False,
        None,
    ),
    (UserProfileReader, "root", (UserProfileV1Reader | UserProfileV2Reader)),
))
