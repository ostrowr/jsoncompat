from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": true,
  "properties": {
    "foo": {
      "type": "string"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}"""
    foo: dc.Omittable[str] = dc.field("foo", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
