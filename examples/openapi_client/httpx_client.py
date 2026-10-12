"""Run both synchronous and asynchronous HTTPX clients against the local server."""

import asyncio

import httpx
from exercise import exercise, exercise_async
from jsoncompat.http import bind, bind_async
from jsoncompat.http.httpx import AsyncHttpxFetch, HttpxFetch, HttpxOptions
from petstore import Health200, HealthRequest, PetstoreAPI
from server import serve


async def async_main(base_url: str) -> None:
    async with httpx.AsyncClient(
        headers={"Authorization": "Bearer example-token"}, timeout=10, trust_env=False
    ) as session:
        client = bind_async(PetstoreAPI, fetch=AsyncHttpxFetch(session, base_url))
        response = await client.request(
            HealthRequest(),
            transport=HttpxOptions(timeout=httpx.Timeout(2, connect=1)),
        )
        assert response.headers.get("content-type") == "application/json; charset=utf-8"
        assert isinstance(response.unwrap(), Health200)
        await exercise_async(client)


def main() -> None:
    with serve() as base_url:
        with httpx.Client(
            headers={"Authorization": "Bearer example-token"},
            timeout=10,
            trust_env=False,
        ) as session:
            client = bind(PetstoreAPI, fetch=HttpxFetch(session, base_url))
            response = client.request(
                HealthRequest(),
                transport=HttpxOptions(timeout=httpx.Timeout(2, connect=1)),
            )
            assert (
                response.headers.get("content-type")
                == "application/json; charset=utf-8"
            )
            assert isinstance(response.unwrap(), Health200)
            exercise(client)
        asyncio.run(async_main(base_url))
    print("httpx: all ten operations passed, sync and async")


if __name__ == "__main__":
    main()
