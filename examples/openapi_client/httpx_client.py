"""Run both synchronous and asynchronous HTTPX clients against the local server."""

import asyncio

import httpx
from exercise import exercise, exercise_async
from jsoncompat.http import bind, bind_async
from petstore import AsyncPetstore, Petstore
from server import serve
from transports import AsyncHttpxFetch, HttpxFetch


async def async_main(base_url: str) -> None:
    async with httpx.AsyncClient(
        headers={"Authorization": "Bearer example-token"}, timeout=10, trust_env=False
    ) as session:
        client = bind_async(AsyncPetstore, fetch=AsyncHttpxFetch(session, base_url))
        await exercise_async(client)


def main() -> None:
    with serve() as base_url:
        with httpx.Client(
            headers={"Authorization": "Bearer example-token"},
            timeout=10,
            trust_env=False,
        ) as session:
            client = bind(Petstore, fetch=HttpxFetch(session, base_url))
            exercise(client)
        asyncio.run(async_main(base_url))
    print("httpx: all ten operations passed, sync and async")


if __name__ == "__main__":
    main()
