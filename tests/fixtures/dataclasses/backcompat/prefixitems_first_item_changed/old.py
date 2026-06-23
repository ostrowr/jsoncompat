from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "minItems": 1,
  "prefixItems": [
    {
      "type": "string"
    }
  ],
  "type": "array"
}"""
    root: collections.abc.Sequence[typing.Any] = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchema, "root", collections.abc.Sequence[typing.Any]),
))
