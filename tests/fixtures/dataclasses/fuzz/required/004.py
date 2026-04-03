from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"required\":[\"__proto__\",\"toString\",\"constructor\"]}"
    __proto__: typing.Any = jsoncompat_dataclasses.jsoncompat_field("__proto__")
    constructor: typing.Any = jsoncompat_dataclasses.jsoncompat_field("constructor")
    toString: typing.Any = jsoncompat_dataclasses.jsoncompat_field("toString")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("__proto__", "__proto__", typing.Any),
    jsoncompat_dataclasses.jsoncompat_field_spec("constructor", "constructor", typing.Any),
    jsoncompat_dataclasses.jsoncompat_field_spec("toString", "toString", typing.Any),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
