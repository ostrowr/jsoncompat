"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "idn-hostname"
}

Tests:
[
  {
    "data": ".",
    "description": "single dot",
    "valid": false
  },
  {
    "data": "。",
    "description": "single ideographic full stop",
    "valid": false
  },
  {
    "data": "．",
    "description": "single fullwidth full stop",
    "valid": false
  },
  {
    "data": "｡",
    "description": "single halfwidth ideographic full stop",
    "valid": false
  },
  {
    "data": "a.b",
    "description": "dot as label separator",
    "valid": true
  },
  {
    "data": "a。b",
    "description": "ideographic full stop as label separator",
    "valid": true
  },
  {
    "data": "a．b",
    "description": "fullwidth full stop as label separator",
    "valid": true
  },
  {
    "data": "a｡b",
    "description": "halfwidth ideographic full stop as label separator",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "idn-hostname"
}
"""

_VALIDATE_FORMATS = False

class Idnhostname1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

