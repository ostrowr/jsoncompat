"""A deterministic local server for the examples and transport integration tests."""

from __future__ import annotations

import threading
from collections.abc import Iterator
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import ClassVar
from urllib.parse import parse_qs, unquote, urlsplit

from jsoncompat import JsonValue, deserialize_json_value, serialize_json_value


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    pet: ClassVar[dict[str, JsonValue]] = {
        "id": "p_123",
        "name": "Mochi",
        "species": "cat",
        "tags": [],
    }

    def log_message(self, format: str, *args: object) -> None:
        pass

    def do_GET(self) -> None:
        self.dispatch()

    def do_POST(self) -> None:
        self.dispatch()

    def do_PATCH(self) -> None:
        self.dispatch()

    def do_PUT(self) -> None:
        self.dispatch()

    def do_DELETE(self) -> None:
        self.dispatch()

    def dispatch(self) -> None:
        target = urlsplit(self.path)
        path = unquote(target.path)
        query = parse_qs(target.query)
        length = int(self.headers.get("Content-Length", "0"))
        payload = self.rfile.read(length)
        if (
            path != "/health"
            and self.headers.get("Authorization") != "Bearer example-token"
        ):
            self.respond(
                401, {"code": "unauthorized", "message": "Example token required"}
            )
            return
        if path == "/health":
            self.respond(200, {"status": "ok"})
        elif path == "/pets/unavailable":
            # Deliberately absent from the OpenAPI contract, like a gateway error.
            self.respond(503, {"message": "Try again later"}, (("Retry-After", "10"),))
        elif path == "/pets" and self.command == "GET":
            self.respond(
                200,
                {"items": [self.pet], "nextOffset": None},
                (("X-Total-Count", "1"),),
            )
        elif path == "/pets" and self.command == "POST":
            value = deserialize_json_value(payload)
            if not isinstance(value, dict):
                self.respond(400, {"code": "body", "message": "Expected an object"})
                return
            self.respond(
                201,
                {**self.pet, **value},
                (("Location", "/pets/p_123"), ("ETag", '"rev-1"')),
            )
        elif path == "/search":
            if "criteria" not in query:
                self.respond(400, {"code": "criteria", "message": "Missing criteria"})
            else:
                deserialize_json_value(query["criteria"][0])
                self.respond(200, [self.pet], (("X-Selected-Fields", "name,species"),))
        elif path.startswith("/groups/"):
            self.respond(200, [self.pet])
        elif path == "/pets/p_123/note":
            if self.command == "PUT":
                value = (
                    payload.decode()
                    if self.headers.get("Content-Type") == "text/plain"
                    else deserialize_json_value(payload)
                )
                self.respond(200, value)
            elif self.headers.get("Accept", "").startswith("text/plain"):
                self.respond(200, "A friendly cat", content_type="text/plain")
            else:
                self.respond(200, "A friendly cat")
        elif path == "/pets/p_123":
            if self.command == "DELETE":
                self.respond(204)
            elif (
                self.command == "GET" and self.headers.get("If-None-Match") == '"rev-1"'
            ):
                self.respond(304)
            elif self.command == "PATCH" and self.headers.get("If-Match") == '"stale"':
                self.respond(412, {"code": "precondition", "message": "ETag changed"})
            else:
                patch_value: JsonValue = (
                    deserialize_json_value(payload) if payload else {}
                )
                if not isinstance(patch_value, dict):
                    self.respond(400, {"code": "body", "message": "Expected an object"})
                    return
                self.respond(200, {**self.pet, **patch_value}, (("ETag", '"rev-1"'),))
        else:
            self.respond(404, {"code": "not_found", "message": "Pet not found"})

    def respond(
        self,
        status: int,
        value: JsonValue = None,
        headers: tuple[tuple[str, str], ...] = (),
        *,
        content_type: str = "application/json",
    ) -> None:
        body = (
            b""
            if status in (204, 304)
            else (
                str(value).encode()
                if content_type == "text/plain"
                else serialize_json_value(value).encode()
            )
        )
        self.send_response(status)
        if status not in (204, 304):
            self.send_header("Content-Type", content_type + "; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("X-Trace", "first")
        self.send_header("X-Trace", "second")
        for name, val in headers:
            self.send_header(name, val)
        self.end_headers()
        self.wfile.write(body)


@contextmanager
def serve() -> Iterator[str]:
    with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            yield f"http://127.0.0.1:{server.server_port}"
        finally:
            server.shutdown()
            thread.join()


if __name__ == "__main__":
    with serve() as base_url:
        print(base_url, flush=True)
        threading.Event().wait()
