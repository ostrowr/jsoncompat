"""The same typed workflow works with any conforming client."""

from typing import Never

from jsoncompat import MISSING
from jsoncompat.http import (
    AsyncClient,
    Client,
    ClientResponse,
    Err,
    HTTPStatusError,
    RequestOptions,
    UnexpectedResponse,
)
from petstore import (
    CreatePet201,
    CreatePetRequest,
    DeletePetRequest,
    GetGroupPetsRequest,
    GetPet304,
    GetPet404,
    GetPetError,
    GetPetNote200ApplicationJson,
    GetPetNoteRequest,
    GetPetRequest,
    GetPetSuccess,
    Health200,
    HealthRequest,
    ListPets200,
    ListPetsRequest,
    PetCreate,
    PetFilter,
    PetPatch,
    PetstoreAPI,
    SearchCriteria,
    SearchPetsRequest,
    SetPetNoteBodyApplicationJson,
    SetPetNoteBodyTextPlain,
    SetPetNoteRequest,
    UpdatePet412,
    UpdatePetRequest,
)


def check_unavailable(response: ClientResponse[GetPetSuccess, GetPetError]) -> None:
    # Metadata is available before attempting model decoding.
    assert response.status == 503
    assert response.headers.get("retry-after") == "10"
    assert response.headers.get_all("X-Trace") == ("first", "second")
    match response.result():
        case Err(error=UnexpectedResponse(status=503) as error):
            assert error.json() == {"message": "Try again later"}
            assert "undeclared HTTP status 503" in str(error.decode_error)
        case _:
            raise AssertionError("expected an undeclared 503")
    try:
        response.unwrap()
    except HTTPStatusError as error:
        assert error.response is response.raw
    else:
        raise AssertionError("unwrap() should raise for an undeclared 503")


def exercise(client: Client[PetstoreAPI, Never]) -> None:
    check_unavailable(client.request(GetPetRequest(pet_id="unavailable")))
    page = client.request(
        ListPetsRequest(
            limit=10,
            tags=["cat", "a&b"],
            filter=PetFilter(adopted=False),
            locale="en",
        )
    ).unwrap()
    # A default response can also match 2xx, so this success union still narrows.
    assert isinstance(page, ListPets200) and page.headers.x_total_count == 1
    assert page.body.items[0].nickname is MISSING
    created = client.request(
        CreatePetRequest(body=PetCreate(name="Mochi", species="cat"))
    ).unwrap()
    assert (
        isinstance(created, CreatePet201) and created.headers.location == "/pets/p_123"
    )
    found = client.request(
        GetPetRequest(pet_id="p_123", include_history=False),
        options=RequestOptions(extra_headers={"X-Trace-ID": "sync-example"}),
    ).unwrap()
    assert [pair[1] for pair in found.raw_headers if pair[0].lower() == "x-trace"] == [
        "first",
        "second",
    ]
    cached = client.request(
        GetPetRequest(pet_id="p_123", if_none_match=found.headers.etag)
    ).result()
    assert isinstance(cached, Err)
    assert isinstance(cached.error, GetPet304) and cached.error.body is None
    missing = client.request(GetPetRequest(pet_id="unknown")).result()
    match missing:
        case Err(error=GetPet404(body=problem)):
            assert problem.code == "not_found"
        case _:
            raise AssertionError("expected the declared 404")
    # The same result can be converted to an exception after inspection.
    assert isinstance(missing, Err)
    try:
        missing.unwrap()
    except HTTPStatusError as error:
        assert error.error is missing.error and error.response.status == 404
    else:
        raise AssertionError("unwrap() should raise for a 404")
    updated = client.request(
        UpdatePetRequest(
            pet_id="p_123",
            body=PetPatch(nickname=None),
            if_match=found.headers.etag,
        )
    ).unwrap()
    assert updated.body.nickname is None
    conflict = client.request(
        UpdatePetRequest(pet_id="p_123", if_match='"stale"')
    ).result()
    assert isinstance(conflict, Err) and isinstance(conflict.error, UpdatePet412)
    note = client.request(GetPetNoteRequest(pet_id="p_123")).unwrap()
    assert (
        isinstance(note, GetPetNote200ApplicationJson) and note.body == "A friendly cat"
    )
    saved = client.request(
        SetPetNoteRequest(
            pet_id="p_123",
            body=SetPetNoteBodyTextPlain(content_type="text/plain", value="Hello!"),
        )
    ).unwrap()
    assert saved.body == "Hello!"
    cleared = client.request(
        SetPetNoteRequest(
            pet_id="p_123",
            body=SetPetNoteBodyApplicationJson(
                content_type="application/json", value=None
            ),
        )
    ).unwrap()
    assert cleared.body is None
    matches = client.request(
        SearchPetsRequest(
            criteria=SearchCriteria(name="Mochi"),
            fields=["name", "species"],
            x_flags=["a", "b"],
        )
    ).unwrap()
    assert matches.headers.x_selected_fields == [
        "name",
        "species",
    ]
    assert (
        client.request(GetGroupPetsRequest(group_id="shelter", pet_ids=[1, 2]))
        .unwrap()
        .status
        == 200
    )
    assert isinstance(client.request(HealthRequest()).unwrap(), Health200)
    assert client.request(DeletePetRequest(pet_id="p_123")).unwrap().body is None


async def exercise_async(client: AsyncClient[PetstoreAPI, Never]) -> None:
    check_unavailable(await client.request(GetPetRequest(pet_id="unavailable")))
    page = (
        await client.request(
            ListPetsRequest(
                limit=10,
                tags=["cat", "a&b"],
                filter=PetFilter(adopted=False),
                locale="en",
            )
        )
    ).unwrap()
    assert isinstance(page, ListPets200) and page.headers.x_total_count == 1
    assert page.body.items[0].nickname is MISSING
    created = (
        await client.request(
            CreatePetRequest(body=PetCreate(name="Mochi", species="cat"))
        )
    ).unwrap()
    assert (
        isinstance(created, CreatePet201) and created.headers.location == "/pets/p_123"
    )
    found = (
        await client.request(
            GetPetRequest(pet_id="p_123", include_history=False),
            options=RequestOptions(extra_headers={"X-Trace-ID": "async-example"}),
        )
    ).unwrap()
    assert [pair[1] for pair in found.raw_headers if pair[0].lower() == "x-trace"] == [
        "first",
        "second",
    ]
    cached = (
        await client.request(
            GetPetRequest(pet_id="p_123", if_none_match=found.headers.etag)
        )
    ).result()
    assert isinstance(cached, Err)
    assert isinstance(cached.error, GetPet304) and cached.error.body is None
    missing = (await client.request(GetPetRequest(pet_id="unknown"))).result()
    match missing:
        case Err(error=GetPet404(body=problem)):
            assert problem.code == "not_found"
        case _:
            raise AssertionError("expected the declared 404")
    # The same result can be converted to an exception after inspection.
    assert isinstance(missing, Err)
    try:
        missing.unwrap()
    except HTTPStatusError as error:
        assert error.error is missing.error and error.response.status == 404
    else:
        raise AssertionError("unwrap() should raise for a 404")
    updated = (
        await client.request(
            UpdatePetRequest(
                pet_id="p_123",
                body=PetPatch(nickname=None),
                if_match=found.headers.etag,
            )
        )
    ).unwrap()
    assert updated.body.nickname is None
    conflict = (
        await client.request(UpdatePetRequest(pet_id="p_123", if_match='"stale"'))
    ).result()
    assert isinstance(conflict, Err) and isinstance(conflict.error, UpdatePet412)
    note = (await client.request(GetPetNoteRequest(pet_id="p_123"))).unwrap()
    assert (
        isinstance(note, GetPetNote200ApplicationJson) and note.body == "A friendly cat"
    )
    saved = (
        await client.request(
            SetPetNoteRequest(
                pet_id="p_123",
                body=SetPetNoteBodyTextPlain(content_type="text/plain", value="Hello!"),
            )
        )
    ).unwrap()
    assert saved.body == "Hello!"
    cleared = (
        await client.request(
            SetPetNoteRequest(
                pet_id="p_123",
                body=SetPetNoteBodyApplicationJson(
                    content_type="application/json", value=None
                ),
            )
        )
    ).unwrap()
    assert cleared.body is None
    matches = (
        await client.request(
            SearchPetsRequest(
                criteria=SearchCriteria(name="Mochi"),
                fields=["name", "species"],
                x_flags=["a", "b"],
            )
        )
    ).unwrap()
    assert matches.headers.x_selected_fields == [
        "name",
        "species",
    ]
    assert (
        await client.request(GetGroupPetsRequest(group_id="shelter", pet_ids=[1, 2]))
    ).unwrap().status == 200
    assert isinstance((await client.request(HealthRequest())).unwrap(), Health200)
    assert (
        await client.request(DeletePetRequest(pet_id="p_123"))
    ).unwrap().body is None
