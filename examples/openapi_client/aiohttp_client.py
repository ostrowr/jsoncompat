"""Run an aiohttp session with its own timeout, credentials, and connection pool."""

import asyncio

import aiohttp
from exercise import exercise_async
from jsoncompat.http import bind_async
from jsoncompat.http.aiohttp import AiohttpFetch, AiohttpOptions
from petstore import Health200, HealthRequest, PetstoreAPI
from server import serve


async def main() -> None:
    with serve() as base_url:
        async with aiohttp.ClientSession(
            headers={"Authorization": "Bearer example-token"},
            timeout=aiohttp.ClientTimeout(total=10),
        ) as session:
            client = bind_async(PetstoreAPI, fetch=AiohttpFetch(session, base_url))
            result = (
                await client.request(
                    HealthRequest(),
                    transport=AiohttpOptions(timeout=aiohttp.ClientTimeout(total=2)),
                )
            ).unwrap()
            assert isinstance(result, Health200)
            await exercise_async(client)
    print("aiohttp: all ten operations passed")


if __name__ == "__main__":
    asyncio.run(main())
