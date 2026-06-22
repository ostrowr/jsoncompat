"""Shared Pydantic configuration for generated fixture benchmark models."""

from pydantic import BaseModel, ConfigDict


class StrictBaseModel(BaseModel):
    """Match jsoncompat's strict, immutable generated-model defaults."""

    model_config = ConfigDict(extra="allow", frozen=True, strict=True)
