from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"required\":[\"foo\\nbar\",\"foo\\\"bar\",\"foo\\\\bar\",\"foo\\rbar\",\"foo\\tbar\",\"foo\\fbar\"]}"
    foo_bar: typing.Any = jsoncompat_dataclasses.jsoncompat_field("foo\tbar")
    foo_bar2: typing.Any = jsoncompat_dataclasses.jsoncompat_field("foo\nbar")
    foo_bar3: typing.Any = jsoncompat_dataclasses.jsoncompat_field("foo\fbar")
    foo_bar4: typing.Any = jsoncompat_dataclasses.jsoncompat_field("foo\rbar")
    foo_bar5: typing.Any = jsoncompat_dataclasses.jsoncompat_field("foo\"bar")
    foo_bar6: typing.Any = jsoncompat_dataclasses.jsoncompat_field("foo\\bar")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar", "foo\tbar", typing.Any),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar2", "foo\nbar", typing.Any),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar3", "foo\fbar", typing.Any),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar4", "foo\rbar", typing.Any),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar5", "foo\"bar", typing.Any),
    jsoncompat_dataclasses.jsoncompat_field_spec("foo_bar6", "foo\\bar", typing.Any),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
