from __future__ import annotations

from typing import Any

from pydantic import BaseModel, ConfigDict, RootModel

class SerializerBase(BaseModel):
    model_config = ConfigDict(
        strict=True,
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

    def model_dump(self, **kwargs):
        kwargs.setdefault("exclude_unset", True)
        return super().model_dump(**kwargs)

    def model_dump_json(self, **kwargs):
        kwargs.setdefault("exclude_unset", True)
        return super().model_dump_json(**kwargs)

class DeserializerBase(BaseModel):
    model_config = ConfigDict(
        strict=True,
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

class SerializerRootModel(RootModel[Any]):
    model_config = ConfigDict(
        strict=True,
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

    def model_dump(self, **kwargs):
        kwargs.setdefault("exclude_unset", True)
        return super().model_dump(**kwargs)

    def model_dump_json(self, **kwargs):
        kwargs.setdefault("exclude_unset", True)
        return super().model_dump_json(**kwargs)

class DeserializerRootModel(RootModel[Any]):
    model_config = ConfigDict(
        strict=True,
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

