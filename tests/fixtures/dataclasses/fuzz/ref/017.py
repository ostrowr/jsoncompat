from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaX(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"x\":{\"$id\":\"http://example.com/b/c.json\",\"not\":{\"$defs\":{\"y\":{\"$id\":\"d.json\",\"type\":\"number\"}}}}},\"$id\":\"http://example.com/b/c.json\",\"not\":{\"$defs\":{\"y\":{\"$id\":\"d.json\",\"type\":\"number\"}}}}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"x\":{\"$id\":\"http://example.com/b/c.json\",\"not\":{\"$defs\":{\"y\":{\"$id\":\"d.json\",\"type\":\"number\"}}}}},\"$id\":\"http://example.com/a.json\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"allOf\":[{\"$ref\":\"http://example.com/b/d.json\"}]}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaX.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
