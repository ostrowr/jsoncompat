"""Opt-in live tests: 24 HTTPS requests to JSONPlaceholder, with simulated writes."""

from __future__ import annotations

import unittest
from typing import TypedDict

import aiohttp
import httpx
import requests
from jsoncompat import MISSING
from jsoncompat.http import AsyncClient, Client, bind, bind_async
from jsonplaceholder import (
    CreatePost201,
    CreatePostRequest,
    GetPost200,
    GetPost404,
    GetPostRequest,
    GetPostResponse,
    JsonPlaceholderAPI,
    ListPosts200,
    ListPostsRequest,
    NewPost,
    PatchPost200,
    PatchPostRequest,
    Post,
    PostPatch,
)
from transports import (
    AiohttpFetch,
    AiohttpOptions,
    AsyncHttpxFetch,
    HttpxFetch,
    HttpxOptions,
    RequestsFetch,
    RequestsOptions,
)

BASE_URL = "https://jsonplaceholder.typicode.com"
TIMEOUT = 20
TITLE = "jsoncompat live test: café ☃"
BODY = "A simulated post.\nNo persistent data is changed."


class TitleUpdate(TypedDict):
    title: str


def check_get(response: GetPostResponse) -> Post:
    assert isinstance(response, GetPost200), response
    assert response.status == 200
    assert response.body.id == 1
    assert response.body.userId == 1
    assert response.body.title and response.body.body
    return response.body


def check_list(response: ListPosts200) -> None:
    assert response.status == 200
    assert len(response.body) == 10
    assert all(post.userId == 1 for post in response.body)
    assert any(post.id == 1 for post in response.body)


def new_post() -> CreatePostRequest:
    return CreatePostRequest(body=NewPost(userId=1, title=TITLE, body=BODY))


def check_create(response: CreatePost201) -> None:
    assert response.status == 201
    assert response.body.to_value() == {
        "id": 101,
        "userId": 1,
        "title": TITLE,
        "body": BODY,
    }


def patch_post() -> PatchPostRequest:
    patch = PostPatch(**TitleUpdate(title=TITLE))
    assert patch.body is MISSING and patch.userId is MISSING
    assert patch.to_value() == {"title": TITLE}
    return PatchPostRequest(post_id=1, body=patch)


def check_patch(response: PatchPost200, original: Post) -> None:
    assert response.status == 200
    original_value = original.to_value()
    assert isinstance(original_value, dict)
    assert response.body.to_value() == {**original_value, "title": TITLE}


def check_missing(response: GetPostResponse) -> None:
    assert isinstance(response, GetPost404), response
    assert response.status == 404
    assert response.body.to_value() == {}


def exercise[Options](
    client: Client[JsonPlaceholderAPI, Options], transport: Options
) -> None:
    original = check_get(client.request(GetPostRequest(post_id=1), transport=transport))
    check_list(client.request(ListPostsRequest(user_id=1)))
    check_create(client.request(new_post()))
    check_patch(client.request(patch_post()), original)
    # JSONPlaceholder simulates writes: a fresh read must still match the original.
    assert check_get(client.request(GetPostRequest(post_id=1))) == original
    check_missing(client.request(GetPostRequest(post_id=999999)))


async def exercise_async[Options](
    client: AsyncClient[JsonPlaceholderAPI, Options], transport: Options
) -> None:
    original = check_get(
        await client.request(GetPostRequest(post_id=1), transport=transport)
    )
    check_list(await client.request(ListPostsRequest(user_id=1)))
    check_create(await client.request(new_post()))
    check_patch(await client.request(patch_post()), original)
    assert check_get(await client.request(GetPostRequest(post_id=1))) == original
    check_missing(await client.request(GetPostRequest(post_id=999999)))


class LiveSyncTests(unittest.TestCase):
    def test_requests(self) -> None:
        with requests.Session() as session:
            exercise(
                bind(
                    JsonPlaceholderAPI,
                    fetch=RequestsFetch(session, BASE_URL, timeout=TIMEOUT),
                ),
                RequestsOptions(timeout=(TIMEOUT, TIMEOUT)),
            )

    def test_httpx(self) -> None:
        with httpx.Client(timeout=TIMEOUT) as session:
            exercise(
                bind(JsonPlaceholderAPI, fetch=HttpxFetch(session, BASE_URL)),
                HttpxOptions(timeout=httpx.Timeout(TIMEOUT)),
            )


class LiveAsyncTests(unittest.IsolatedAsyncioTestCase):
    async def test_aiohttp(self) -> None:
        async with aiohttp.ClientSession(
            timeout=aiohttp.ClientTimeout(total=TIMEOUT), trust_env=True
        ) as session:
            await exercise_async(
                bind_async(JsonPlaceholderAPI, fetch=AiohttpFetch(session, BASE_URL)),
                AiohttpOptions(timeout=aiohttp.ClientTimeout(total=TIMEOUT)),
            )

    async def test_httpx(self) -> None:
        async with httpx.AsyncClient(timeout=TIMEOUT) as session:
            await exercise_async(
                bind_async(
                    JsonPlaceholderAPI, fetch=AsyncHttpxFetch(session, BASE_URL)
                ),
                HttpxOptions(timeout=httpx.Timeout(TIMEOUT)),
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
