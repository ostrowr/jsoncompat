from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0Branch0(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"allOf\":[{\"multipleOf\":2}],\"multipleOf\":3}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"allOf\":[{\"multipleOf\":2}],\"anyOf\":[{\"multipleOf\":3}],\"multipleOf\":5}"
    root: GeneratedSchemaBranch0Branch0 = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"allOf\":[{\"multipleOf\":2}],\"anyOf\":[{\"multipleOf\":3}],\"oneOf\":[{\"multipleOf\":5}]}"
    root: GeneratedSchemaBranch0 = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch0Branch0.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch0.__jsoncompat_root_annotation__ = GeneratedSchemaBranch0Branch0

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaBranch0

JSONCOMPAT_MODEL = GeneratedSchema
