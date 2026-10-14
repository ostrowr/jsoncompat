"""Use a prepared request with HTTPX's own per-call transport controls."""

import httpx
from jsoncompat.http import RequestOptions, prepare
from jsoncompat.http.httpx import HttpxFetch, HttpxOptions
from petstore.api import PetstoreAPI
from petstore.operations.pets import GetPetRequest
from server import serve


def main() -> None:
    call = prepare(
        PetstoreAPI,
        GetPetRequest(pet_id="p_123"),
        options=RequestOptions(
            extra_headers={
                "Authorization": "Bearer example-token",
                "X-Trace-ID": "example",
            },
            extra_query=(("preview", "true"),),
        ),
    )
    with serve() as base_url, httpx.Client(trust_env=False) as session:
        # Inspect the buffered response before deciding whether to decode it.
        fetch = HttpxFetch(session, base_url)
        raw = fetch(
            call.request, transport=HttpxOptions(timeout=httpx.Timeout(5, connect=1))
        )
        assert raw.status == 200 and raw.body
        result = call.decode(raw).unwrap()  # GetPet200, validated and unwrapped.
        assert result.body.name == "Mochi"
    print(
        "prepared request: transport controls, raw response, and typed decoding passed"
    )


if __name__ == "__main__":
    main()
