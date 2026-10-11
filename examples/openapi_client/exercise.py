"""The same typed workflow works with any conforming client."""

from typing import Never

from jsoncompat import MISSING
from jsoncompat.http import AsyncClient, Client, RequestOptions
from petstore import (
    CreatePet201,
    CreatePetRequest,
    DeletePet204,
    DeletePetRequest,
    GetGroupPetsRequest,
    GetPet200,
    GetPet304,
    GetPet404,
    GetPetNote200ApplicationJson,
    GetPetNoteRequest,
    GetPetRequest,
    Health200,
    HealthRequest,
    ListPets200,
    ListPetsRequest,
    PetCreate,
    PetFilter,
    PetPatch,
    PetstoreAPI,
    SearchCriteria,
    SearchPets200,
    SearchPetsRequest,
    SetPetNote200,
    SetPetNoteBodyApplicationJson,
    SetPetNoteBodyTextPlain,
    SetPetNoteRequest,
    UpdatePet200,
    UpdatePet412,
    UpdatePetRequest,
)


def exercise(client: Client[PetstoreAPI, Never]) -> None:
    page = client.request(
        ListPetsRequest(
            limit=10, tags=["cat", "a&b"], filter=PetFilter(adopted=False), locale="en"
        )
    )
    assert isinstance(page, ListPets200) and page.headers.x_total_count == 1
    assert page.body.items[0].nickname is MISSING
    created = client.request(
        CreatePetRequest(body=PetCreate(name="Mochi", species="cat"))
    )
    assert (
        isinstance(created, CreatePet201) and created.headers.location == "/pets/p_123"
    )
    found = client.request(
        GetPetRequest(pet_id="p_123", include_history=False),
        options=RequestOptions(extra_headers={"X-Trace-ID": "sync-example"}),
    )
    assert isinstance(found, GetPet200)
    assert [pair[1] for pair in found.raw_headers if pair[0].lower() == "x-trace"] == [
        "first",
        "second",
    ]
    cached = client.request(
        GetPetRequest(pet_id="p_123", if_none_match=found.headers.etag)
    )
    assert isinstance(cached, GetPet304) and cached.body is None
    missing = client.request(GetPetRequest(pet_id="unknown"))
    assert isinstance(missing, GetPet404) and missing.body.code == "not_found"
    updated = client.request(
        UpdatePetRequest(
            pet_id="p_123", body=PetPatch(nickname=None), if_match=found.headers.etag
        )
    )
    assert isinstance(updated, UpdatePet200) and updated.body.nickname is None
    conflict = client.request(UpdatePetRequest(pet_id="p_123", if_match='"stale"'))
    assert isinstance(conflict, UpdatePet412)
    note = client.request(GetPetNoteRequest(pet_id="p_123"))
    assert (
        isinstance(note, GetPetNote200ApplicationJson) and note.body == "A friendly cat"
    )
    saved = client.request(
        SetPetNoteRequest(
            pet_id="p_123",
            body=SetPetNoteBodyTextPlain(content_type="text/plain", value="Hello!"),
        )
    )
    assert isinstance(saved, SetPetNote200) and saved.body == "Hello!"
    cleared = client.request(
        SetPetNoteRequest(
            pet_id="p_123",
            body=SetPetNoteBodyApplicationJson(
                content_type="application/json", value=None
            ),
        )
    )
    assert isinstance(cleared, SetPetNote200) and cleared.body is None
    matches = client.request(
        SearchPetsRequest(
            criteria=SearchCriteria(name="Mochi"),
            fields=["name", "species"],
            x_flags=["a", "b"],
        )
    )
    assert isinstance(matches, SearchPets200) and matches.headers.x_selected_fields == [
        "name",
        "species",
    ]
    assert (
        client.request(GetGroupPetsRequest(group_id="shelter", pet_ids=[1, 2])).status
        == 200
    )
    assert isinstance(client.request(HealthRequest()), Health200)
    assert isinstance(client.request(DeletePetRequest(pet_id="p_123")), DeletePet204)


async def exercise_async(client: AsyncClient[PetstoreAPI, Never]) -> None:
    page = await client.request(
        ListPetsRequest(
            limit=10, tags=["cat", "a&b"], filter=PetFilter(adopted=False), locale="en"
        )
    )
    assert isinstance(page, ListPets200) and page.headers.x_total_count == 1
    assert page.body.items[0].nickname is MISSING
    created = await client.request(
        CreatePetRequest(body=PetCreate(name="Mochi", species="cat"))
    )
    assert (
        isinstance(created, CreatePet201) and created.headers.location == "/pets/p_123"
    )
    found = await client.request(
        GetPetRequest(pet_id="p_123", include_history=False),
        options=RequestOptions(extra_headers={"X-Trace-ID": "async-example"}),
    )
    assert isinstance(found, GetPet200)
    assert [pair[1] for pair in found.raw_headers if pair[0].lower() == "x-trace"] == [
        "first",
        "second",
    ]
    assert isinstance(
        await client.request(
            GetPetRequest(pet_id="p_123", if_none_match=found.headers.etag)
        ),
        GetPet304,
    )
    missing = await client.request(GetPetRequest(pet_id="unknown"))
    assert isinstance(missing, GetPet404) and missing.body.code == "not_found"
    updated = await client.request(
        UpdatePetRequest(
            pet_id="p_123", body=PetPatch(nickname=None), if_match=found.headers.etag
        )
    )
    assert isinstance(updated, UpdatePet200) and updated.body.nickname is None
    assert isinstance(
        await client.request(UpdatePetRequest(pet_id="p_123", if_match='"stale"')),
        UpdatePet412,
    )
    assert isinstance(
        await client.request(GetPetNoteRequest(pet_id="p_123")),
        GetPetNote200ApplicationJson,
    )
    saved = await client.request(
        SetPetNoteRequest(
            pet_id="p_123",
            body=SetPetNoteBodyTextPlain(content_type="text/plain", value="Hello!"),
        )
    )
    assert isinstance(saved, SetPetNote200) and saved.body == "Hello!"
    cleared = await client.request(
        SetPetNoteRequest(
            pet_id="p_123",
            body=SetPetNoteBodyApplicationJson(
                content_type="application/json", value=None
            ),
        )
    )
    assert isinstance(cleared, SetPetNote200) and cleared.body is None
    matches = await client.request(
        SearchPetsRequest(
            criteria=SearchCriteria(name="Mochi"),
            fields=["name", "species"],
            x_flags=["a", "b"],
        )
    )
    assert isinstance(matches, SearchPets200) and matches.headers.x_selected_fields == [
        "name",
        "species",
    ]
    assert (
        await client.request(GetGroupPetsRequest(group_id="shelter", pet_ids=[1, 2]))
    ).status == 200
    assert isinstance(await client.request(HealthRequest()), Health200)
    assert isinstance(
        await client.request(DeletePetRequest(pet_id="p_123")), DeletePet204
    )
