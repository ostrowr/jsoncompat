"""Run with: uv run --project pybindings --extra requests python examples/openapi_client/requests_client.py"""

import requests
from exercise import exercise
from jsoncompat.http import bind
from jsoncompat.http.requests import RequestsFetch, RequestsOptions
from petstore import Health200, HealthRequest, PetstoreAPI
from server import serve


def main() -> None:
    with serve() as base_url, requests.Session() as session:
        session.trust_env = False
        session.headers["Authorization"] = "Bearer example-token"
        client = bind(PetstoreAPI, fetch=RequestsFetch(session, base_url, timeout=10))
        result = client.request(
            HealthRequest(), transport=RequestsOptions(timeout=(1, 2))
        )
        assert isinstance(result, Health200)
        exercise(client)
    print("requests: all ten operations passed")


if __name__ == "__main__":
    main()
