from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneA(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "true"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOne(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"one\":{\"properties\":{\"a\":true}},\"two\":{\"properties\":{\"x\":true},\"required\":[\"x\"]}},\"properties\":{\"a\":true}}"
    a: (GeneratedSchemaOneA | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("a", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoX(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "true"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwo(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"one\":{\"properties\":{\"a\":true}},\"two\":{\"properties\":{\"x\":true},\"required\":[\"x\"]}},\"properties\":{\"x\":true},\"required\":[\"x\"]}"
    x: GeneratedSchemaTwoX = jsoncompat_dataclasses.jsoncompat_field("x")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"one\":{\"properties\":{\"a\":true}},\"two\":{\"properties\":{\"x\":true},\"required\":[\"x\"]}},\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"allOf\":[{\"$ref\":\"#/$defs/one\"},{\"properties\":{\"b\":true}},{\"oneOf\":[{\"$ref\":\"#/$defs/two\"},{\"properties\":{\"y\":true},\"required\":[\"y\"]}]}],\"unevaluatedProperties\":false}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaOneA.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaOne.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("a", "a", (GeneratedSchemaOneA | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchemaTwoX.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaTwo.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("x", "x", GeneratedSchemaTwoX),
    extra_annotation=dict[str, typing.Any],
)

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
