from typing import Callable
from typing import Protocol


class HasJsonSchema(Protocol):
    def model_json_schema(self) -> dict: ...


def check_compatibility[C: HasJsonSchema](
    stable_id: str, mode: str = "both"
) -> Callable[[C], C]:
    """
    Decorator to check compatibility of a class with a stable ID.

    At runtime, this is a noop; we patch this decorator at test time to actually
    register the schemas.
    """
    return lambda cls: cls
