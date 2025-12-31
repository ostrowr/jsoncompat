"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "json-pointer"
}

Tests:
[
  {
    "data": 12,
    "description": "all string formats ignore integers",
    "valid": true
  },
  {
    "data": 13.7,
    "description": "all string formats ignore floats",
    "valid": true
  },
  {
    "data": {},
    "description": "all string formats ignore objects",
    "valid": true
  },
  {
    "data": [],
    "description": "all string formats ignore arrays",
    "valid": true
  },
  {
    "data": false,
    "description": "all string formats ignore booleans",
    "valid": true
  },
  {
    "data": null,
    "description": "all string formats ignore nulls",
    "valid": true
  },
  {
    "data": "/foo/bar~0/baz~1/%a",
    "description": "a valid JSON-pointer",
    "valid": true
  },
  {
    "data": "/foo/bar~",
    "description": "not a valid JSON-pointer (~ not escaped)",
    "valid": false
  },
  {
    "data": "/foo//bar",
    "description": "valid JSON-pointer with empty segment",
    "valid": true
  },
  {
    "data": "/foo/bar/",
    "description": "valid JSON-pointer with the last empty segment",
    "valid": true
  },
  {
    "data": "",
    "description": "valid JSON-pointer as stated in RFC 6901 #1",
    "valid": true
  },
  {
    "data": "/foo",
    "description": "valid JSON-pointer as stated in RFC 6901 #2",
    "valid": true
  },
  {
    "data": "/foo/0",
    "description": "valid JSON-pointer as stated in RFC 6901 #3",
    "valid": true
  },
  {
    "data": "/",
    "description": "valid JSON-pointer as stated in RFC 6901 #4",
    "valid": true
  },
  {
    "data": "/a~1b",
    "description": "valid JSON-pointer as stated in RFC 6901 #5",
    "valid": true
  },
  {
    "data": "/c%d",
    "description": "valid JSON-pointer as stated in RFC 6901 #6",
    "valid": true
  },
  {
    "data": "/e^f",
    "description": "valid JSON-pointer as stated in RFC 6901 #7",
    "valid": true
  },
  {
    "data": "/g|h",
    "description": "valid JSON-pointer as stated in RFC 6901 #8",
    "valid": true
  },
  {
    "data": "/i\\j",
    "description": "valid JSON-pointer as stated in RFC 6901 #9",
    "valid": true
  },
  {
    "data": "/k\"l",
    "description": "valid JSON-pointer as stated in RFC 6901 #10",
    "valid": true
  },
  {
    "data": "/ ",
    "description": "valid JSON-pointer as stated in RFC 6901 #11",
    "valid": true
  },
  {
    "data": "/m~0n",
    "description": "valid JSON-pointer as stated in RFC 6901 #12",
    "valid": true
  },
  {
    "data": "/foo/-",
    "description": "valid JSON-pointer used adding to the last array position",
    "valid": true
  },
  {
    "data": "/foo/-/bar",
    "description": "valid JSON-pointer (- used as object member name)",
    "valid": true
  },
  {
    "data": "/~1~0~0~1~1",
    "description": "valid JSON-pointer (multiple escaped characters)",
    "valid": true
  },
  {
    "data": "/~1.1",
    "description": "valid JSON-pointer (escaped with fraction part) #1",
    "valid": true
  },
  {
    "data": "/~0.1",
    "description": "valid JSON-pointer (escaped with fraction part) #2",
    "valid": true
  },
  {
    "data": "#",
    "description": "not a valid JSON-pointer (URI Fragment Identifier) #1",
    "valid": false
  },
  {
    "data": "#/",
    "description": "not a valid JSON-pointer (URI Fragment Identifier) #2",
    "valid": false
  },
  {
    "data": "#a",
    "description": "not a valid JSON-pointer (URI Fragment Identifier) #3",
    "valid": false
  },
  {
    "data": "/~0~",
    "description": "not a valid JSON-pointer (some escaped, but not all) #1",
    "valid": false
  },
  {
    "data": "/~0/~",
    "description": "not a valid JSON-pointer (some escaped, but not all) #2",
    "valid": false
  },
  {
    "data": "/~2",
    "description": "not a valid JSON-pointer (wrong escape character) #1",
    "valid": false
  },
  {
    "data": "/~-1",
    "description": "not a valid JSON-pointer (wrong escape character) #2",
    "valid": false
  },
  {
    "data": "/~~",
    "description": "not a valid JSON-pointer (multiple characters not escaped)",
    "valid": false
  },
  {
    "data": "a",
    "description": "not a valid JSON-pointer (isn't empty nor starts with /) #1",
    "valid": false
  },
  {
    "data": "0",
    "description": "not a valid JSON-pointer (isn't empty nor starts with /) #2",
    "valid": false
  },
  {
    "data": "a/a",
    "description": "not a valid JSON-pointer (isn't empty nor starts with /) #3",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Jsonpointer0Serializer(SerializerRootModel):
    root: Any

