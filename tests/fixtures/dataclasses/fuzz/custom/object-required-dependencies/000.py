from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"dependentRequired\":{\"name\":[\"email\"]},\"minProperties\":2,\"properties\":{\"email\":{\"type\":\"string\"},\"kind\":{\"const\":\"user\"},\"name\":{\"type\":\"string\"}},\"required\":[\"kind\",\"email\"],\"type\":\"object\"}"
    email: str = jsoncompat_dataclasses.jsoncompat_field("email")
    kind: typing.Literal["user"] = jsoncompat_dataclasses.jsoncompat_field("kind")
    name: (str | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("name", omittable=True)

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("email", "email", str),
    jsoncompat_dataclasses.jsoncompat_field_spec("kind", "kind", typing.Literal["user"]),
    jsoncompat_dataclasses.jsoncompat_field_spec("name", "name", (str | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
)

JSONCOMPAT_MODEL = GeneratedSchema
