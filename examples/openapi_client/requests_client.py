"""Run with: uv run --project pybindings --group http-examples python examples/openapi_client/requests_client.py"""

import requests
from exercise import exercise
from jsoncompat.http import bind
from petstore import PetstoreAPI
from server import serve
from transports import RequestsFetch


def main() -> None:
    with serve() as base_url, requests.Session() as session:
        session.trust_env = False
        session.headers["Authorization"] = "Bearer example-token"
        client = bind(PetstoreAPI, fetch=RequestsFetch(session, base_url, timeout=10))
        exercise(client)
    print("requests: all ten operations passed")


if __name__ == "__main__":
    main()
