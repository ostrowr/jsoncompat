from decorator import check_compatibility
import pydantic


@check_compatibility("x1", mode="serializer")
class X(pydantic.BaseModel):
    x: int


@check_compatibility("x2", mode="serializer")
class X2(pydantic.BaseModel):
    x: int
