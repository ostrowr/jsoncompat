"""Isolated wheel smoke test, invoked by check_http_extras.py for each extra."""

import asyncio
import sys
from importlib import import_module, util
from pathlib import Path

import jsoncompat
from jsoncompat.http import EncodedRequest, Endpoint, RawResponse


def main() -> None:
    extra = sys.argv[1]
    assert jsoncompat.__file__ is not None
    assert (
        Path(jsoncompat.__file__).resolve().is_relative_to(Path(sys.prefix).resolve())
    )
    clients = {"aiohttp", "httpx", "requests"}
    assert not clients.intersection(sys.modules), "core imported an optional client"
    for name in clients:
        if name != extra:
            assert util.find_spec(name) is None, f"unexpected dependency: {name}"
            try:
                import_module(f"jsoncompat.http.{name}")
            except ModuleNotFoundError as error:
                assert error.name == name
            else:
                raise AssertionError(f"{name} imported without its optional dependency")
    if extra == "core":
        print("core wheel imports with no HTTP clients installed")
        return

    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
    from threading import Thread

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args: object) -> None:
            pass

        def do_GET(self) -> None:
            assert self.path == "/v1/a%2Fb?q=x&q=y"
            if extra != "requests":
                assert self.headers.get_all("X-Repeated") == ["one", "two"]
            self.send_response(404)
            self.send_header("Content-Length", "2")
            self.send_header("Set-Cookie", "a=1")
            self.send_header("Set-Cookie", "b=2")
            self.end_headers()
            self.wfile.write(b"{}")

    def check(response: RawResponse) -> None:
        assert response.status == 404 and response.body == b"{}"
        assert [v for k, v in response.headers if k.lower() == "set-cookie"] == [
            "a=1",
            "b=2",
        ]

    request = EncodedRequest(
        "GET",
        "/a%2Fb?q=x&q=y",
        () if extra == "requests" else (("X-Repeated", "one"), ("x-repeated", "two")),
        None,
        Endpoint("GET", "/{id}", ()),
    )
    adapter = import_module(f"jsoncompat.http.{extra}")
    with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        url = f"http://127.0.0.1:{server.server_port}/v1/"
        try:
            if extra == "aiohttp":
                import aiohttp

                async def run_aiohttp() -> None:
                    async with aiohttp.ClientSession(raise_for_status=True) as session:
                        fetch = adapter.AiohttpFetch(session, url)
                        check(
                            await fetch(
                                request,
                                transport=adapter.AiohttpOptions(
                                    timeout=aiohttp.ClientTimeout(total=5)
                                ),
                            )
                        )
                        assert not session.closed

                asyncio.run(run_aiohttp())
            elif extra == "httpx":
                import httpx

                with httpx.Client(trust_env=False) as session:
                    check(
                        adapter.HttpxFetch(session, url)(
                            request, transport=adapter.HttpxOptions(timeout=5)
                        )
                    )
                    assert not session.is_closed

                async def run_httpx() -> None:
                    async with httpx.AsyncClient(trust_env=False) as session:
                        check(await adapter.AsyncHttpxFetch(session, url)(request))
                        assert not session.is_closed

                asyncio.run(run_httpx())
            elif extra == "requests":
                import requests

                with requests.Session() as session:
                    session.trust_env = False
                    check(
                        adapter.RequestsFetch(session, url)(
                            request, transport=adapter.RequestsOptions(timeout=(5, 5))
                        )
                    )
        finally:
            server.shutdown()
            thread.join()
    assert clients.intersection(sys.modules) == {extra}, (
        "integration imported another HTTP client"
    )
    print(f"{extra} wheel extra passed its isolated socket test")


if __name__ == "__main__":
    main()
