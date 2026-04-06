from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"anyOf\":[{\"enum\":[null]}]}"
    root: typing.Literal[None] = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"anyOf\":[{\"anyOf\":[{\"enum\":[null]}]}]}"
    root: GeneratedSchemaBranch0 = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch0.__jsoncompat_root_annotation__ = typing.Literal[None]

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaBranch0

JSONCOMPAT_MODEL = GeneratedSchema
