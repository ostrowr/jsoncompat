from demo.decorator import check_compatibility
import pydantic


@check_compatibility("simple_int", mode="serializer")
class X(pydantic.BaseModel):
    x: int


@check_compatibility("simple_str", mode="serializer")
class X2(pydantic.BaseModel):
    x: str


@check_compatibility("simple_optional_str", mode="deserializer")
class X3(pydantic.BaseModel):
    x: str | None = pydantic.Field(default=None, min_length=3)


@check_compatibility("several_fields", mode="both")
class X4(pydantic.BaseModel):
    x: str | None = pydantic.Field(default=None, min_length=3)
    y: int
    z: float | None = None
    name: str = pydantic.Field(default="default", max_length=20)
    active: bool = True
    tags: list[str] = pydantic.Field(default_factory=list)
