from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "children": {
      "items": {
        "$ref": "#"
      },
      "type": "array"
    },
    "metadata": {
      "type": "string"
    },
    "value": {
      "type": "integer"
    }
  },
  "required": [
    "value"
  ],
  "type": "object"
}"""
    children: dc.Omittable[collections.abc.Sequence[GeneratedSchema]] = dc.field("children", omittable=True)
    metadata: dc.Omittable[str] = dc.field("metadata", omittable=True)
    value: int = dc.field("value")

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (
        GeneratedSchema,
        "object",
        (
            ("children", "children", collections.abc.Sequence[GeneratedSchema], True),
            ("metadata", "metadata", str, True),
            ("value", "value", int, False),
        ),
        False,
        None,
    ),
))
