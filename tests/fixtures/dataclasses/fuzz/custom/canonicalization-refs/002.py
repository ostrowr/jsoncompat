from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"allOf\":[{\"$ref\":\"#/x/allOf/1/properties/value\"}],\"x\":{\"allOf\":[{\"properties\":{\"value\":{\"type\":\"string\"}},\"type\":\"object\"},{\"properties\":{\"value\":{\"type\":\"string\"}},\"type\":\"object\"}]}}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
