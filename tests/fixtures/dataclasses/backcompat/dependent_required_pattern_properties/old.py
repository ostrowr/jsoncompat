from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"additionalProperties\":false,\"dependentRequired\":{\"x\":[\"y\"]},\"patternProperties\":{\"^x$\":true},\"type\":\"object\"}"
    pass

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
)

JSONCOMPAT_MODEL = GeneratedSchema
