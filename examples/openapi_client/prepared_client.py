"""Use a prepared request with HTTPX's own per-call transport controls."""

import httpx
from jsoncompat.http import EncodedRequest, RawResponse, RequestOptions, prepare
from petstore import GetPet200, GetPetRequest, PetstoreAPI
from server import serve


def fetch(
    session: httpx.Client,
    base_url: str,
    request: EncodedRequest,
    *,
    timeout: httpx.Timeout,
    follow_redirects: bool = False,
) -> RawResponse:
    response = session.request(
        request.method,
        base_url.rstrip("/") + request.target,
        headers=request.headers,
        content=request.body,
        timeout=timeout,
        follow_redirects=follow_redirects,
    )
    return RawResponse(
        response.status_code,
        tuple(
            (key.decode("ascii"), value.decode("latin-1"))
            for key, value in response.headers.raw
        ),
        response.content,
    )


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
        # Timeout policy belongs to this fetcher; no jsoncompat transport flags.
        raw = fetch(
            session, base_url, call.request, timeout=httpx.Timeout(5, connect=1)
        )
        assert raw.status == 200 and raw.body
        result = call.decode(raw)  # GetPetResponse, validated only when decoded.
        assert isinstance(result, GetPet200) and result.body.name == "Mochi"
    print(
        "prepared request: transport controls, raw response, and typed decoding passed"
    )


if __name__ == "__main__":
    main()
