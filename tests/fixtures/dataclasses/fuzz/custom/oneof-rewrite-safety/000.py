from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"kind\":{\"enum\":[\"int\"]},\"value\":{\"multipleOf\":1,\"type\":\"integer\"}},\"required\":[\"kind\",\"value\"],\"type\":\"object\"}"
    kind: typing.Literal["int"] = jsoncompat_dataclasses.jsoncompat_field("kind")
    value: int = jsoncompat_dataclasses.jsoncompat_field("value")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"kind\":{\"enum\":[\"str\"]},\"value\":{\"minLength\":0,\"type\":\"string\"}},\"required\":[\"kind\",\"value\"],\"type\":\"object\"}"
    kind: typing.Literal["str"] = jsoncompat_dataclasses.jsoncompat_field("kind")
    value: str = jsoncompat_dataclasses.jsoncompat_field("value")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"oneOf\":[{\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"kind\":{\"enum\":[\"int\"]},\"value\":{\"multipleOf\":1,\"type\":\"integer\"}},\"required\":[\"kind\",\"value\"],\"type\":\"object\"},{\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"kind\":{\"enum\":[\"str\"]},\"value\":{\"minLength\":0,\"type\":\"string\"}},\"required\":[\"kind\",\"value\"],\"type\":\"object\"}]}"
    root: (GeneratedSchemaBranch0 | GeneratedSchemaBranch1) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaBranch0.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("kind", "kind", typing.Literal["int"]),
    jsoncompat_dataclasses.jsoncompat_field_spec("value", "value", int),
)

GeneratedSchemaBranch1.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("kind", "kind", typing.Literal["str"]),
    jsoncompat_dataclasses.jsoncompat_field_spec("value", "value", str),
)

GeneratedSchema.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch0 | GeneratedSchemaBranch1)

JSONCOMPAT_MODEL = GeneratedSchema
