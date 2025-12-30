"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "required": [
    "foo\nbar",
    "foo\"bar",
    "foo\\bar",
    "foo\rbar",
    "foo\tbar",
    "foo\fbar"
  ]
}

Tests:
[
  {
    "data": {
      "foo\tbar": 1,
      "foo\nbar": 1,
      "foo\fbar": 1,
      "foo\rbar": 1,
      "foo\"bar": 1,
      "foo\\bar": 1
    },
    "description": "object with all properties present is valid",
    "valid": true
  },
  {
    "data": {
      "foo\nbar": "1",
      "foo\"bar": "1"
    },
    "description": "object with some properties missing is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Required3Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo_bar: Annotated[Any, Field(alias="foo\tbar")]
    foo_bar_2: Annotated[Any, Field(alias="foo\nbar")]
    foo_bar_3: Annotated[Any, Field(alias="foo\fbar")]
    foo_bar_4: Annotated[Any, Field(alias="foo\rbar")]
    foo_bar_5: Annotated[Any, Field(alias="foo\"bar")]
    foo_bar_6: Annotated[Any, Field(alias="foo\\bar")]

