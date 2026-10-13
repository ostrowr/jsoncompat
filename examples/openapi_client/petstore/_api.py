# Generated nominal API identity; importing it does not load models.
from typing import final, override
from jsoncompat import http as _http

@final
class PetstoreAPI(_http.API):
    __slots__ = ()

    @classmethod
    @override
    def operations(cls) -> tuple[_http.Operation, ...]:
        from .api import OPERATIONS
        return OPERATIONS
