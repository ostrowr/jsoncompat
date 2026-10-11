"""The same typed workflow works with any conforming client."""

from jsoncompat import MISSING
from petstore import (
    AsyncPetstore,
    CreatePet201,
    CreatePetInput,
    DeletePet204,
    DeletePetInput,
    GetGroupPetsInput,
    GetPet200,
    GetPet304,
    GetPet404,
    GetPetInput,
    GetPetNote200ApplicationJson,
    GetPetNoteInput,
    Health200,
    HealthInput,
    ListPets200,
    ListPetsInput,
    PetCreate,
    PetFilter,
    PetPatch,
    Petstore,
    SearchCriteria,
    SearchPets200,
    SearchPetsInput,
    SetPetNote200,
    SetPetNoteBodyApplicationJson,
    SetPetNoteBodyTextPlain,
    SetPetNoteInput,
    UpdatePet200,
    UpdatePet412,
    UpdatePetInput,
)


def exercise(client: Petstore) -> None:
    page = client.list_pets(
        ListPetsInput(
            limit=10, tags=["cat", "a&b"], filter=PetFilter(adopted=False), locale="en"
        )
    )
    assert isinstance(page, ListPets200) and page.headers.x_total_count == 1
    assert page.body.items[0].nickname is MISSING
    created = client.create_pet(
        CreatePetInput(body=PetCreate(name="Mochi", species="cat"))
    )
    assert (
        isinstance(created, CreatePet201) and created.headers.location == "/pets/p_123"
    )
    found = client.get_pet(GetPetInput(pet_id="p_123", include_history=False))
    assert isinstance(found, GetPet200)
    assert [pair[1] for pair in found.raw_headers if pair[0].lower() == "x-trace"] == [
        "first",
        "second",
    ]
    cached = client.get_pet(
        GetPetInput(pet_id="p_123", if_none_match=found.headers.etag)
    )
    assert isinstance(cached, GetPet304) and cached.body is None
    missing = client.get_pet(GetPetInput(pet_id="unknown"))
    assert isinstance(missing, GetPet404) and missing.body.code == "not_found"
    updated = client.update_pet(
        UpdatePetInput(
            pet_id="p_123", body=PetPatch(nickname=None), if_match=found.headers.etag
        )
    )
    assert isinstance(updated, UpdatePet200) and updated.body.nickname is None
    conflict = client.update_pet(UpdatePetInput(pet_id="p_123", if_match='"stale"'))
    assert isinstance(conflict, UpdatePet412)
    note = client.get_pet_note(GetPetNoteInput(pet_id="p_123"))
    assert (
        isinstance(note, GetPetNote200ApplicationJson) and note.body == "A friendly cat"
    )
    saved = client.set_pet_note(
        SetPetNoteInput(
            pet_id="p_123",
            body=SetPetNoteBodyTextPlain(content_type="text/plain", value="Hello!"),
        )
    )
    assert isinstance(saved, SetPetNote200) and saved.body == "Hello!"
    cleared = client.set_pet_note(
        SetPetNoteInput(
            pet_id="p_123",
            body=SetPetNoteBodyApplicationJson(
                content_type="application/json", value=None
            ),
        )
    )
    assert isinstance(cleared, SetPetNote200) and cleared.body is None
    matches = client.search_pets(
        SearchPetsInput(
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
        client.get_group_pets(
            GetGroupPetsInput(group_id="shelter", pet_ids=[1, 2])
        ).status
        == 200
    )
    assert isinstance(client.health(HealthInput()), Health200)
    assert isinstance(client.delete_pet(DeletePetInput(pet_id="p_123")), DeletePet204)


async def exercise_async(client: AsyncPetstore) -> None:
    page = await client.list_pets(
        ListPetsInput(
            limit=10, tags=["cat", "a&b"], filter=PetFilter(adopted=False), locale="en"
        )
    )
    assert isinstance(page, ListPets200) and page.headers.x_total_count == 1
    assert page.body.items[0].nickname is MISSING
    created = await client.create_pet(
        CreatePetInput(body=PetCreate(name="Mochi", species="cat"))
    )
    assert (
        isinstance(created, CreatePet201) and created.headers.location == "/pets/p_123"
    )
    found = await client.get_pet(GetPetInput(pet_id="p_123", include_history=False))
    assert isinstance(found, GetPet200)
    assert [pair[1] for pair in found.raw_headers if pair[0].lower() == "x-trace"] == [
        "first",
        "second",
    ]
    assert isinstance(
        await client.get_pet(
            GetPetInput(pet_id="p_123", if_none_match=found.headers.etag)
        ),
        GetPet304,
    )
    missing = await client.get_pet(GetPetInput(pet_id="unknown"))
    assert isinstance(missing, GetPet404) and missing.body.code == "not_found"
    updated = await client.update_pet(
        UpdatePetInput(
            pet_id="p_123", body=PetPatch(nickname=None), if_match=found.headers.etag
        )
    )
    assert isinstance(updated, UpdatePet200) and updated.body.nickname is None
    assert isinstance(
        await client.update_pet(UpdatePetInput(pet_id="p_123", if_match='"stale"')),
        UpdatePet412,
    )
    assert isinstance(
        await client.get_pet_note(GetPetNoteInput(pet_id="p_123")),
        GetPetNote200ApplicationJson,
    )
    saved = await client.set_pet_note(
        SetPetNoteInput(
            pet_id="p_123",
            body=SetPetNoteBodyTextPlain(content_type="text/plain", value="Hello!"),
        )
    )
    assert isinstance(saved, SetPetNote200) and saved.body == "Hello!"
    cleared = await client.set_pet_note(
        SetPetNoteInput(
            pet_id="p_123",
            body=SetPetNoteBodyApplicationJson(
                content_type="application/json", value=None
            ),
        )
    )
    assert isinstance(cleared, SetPetNote200) and cleared.body is None
    matches = await client.search_pets(
        SearchPetsInput(
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
        await client.get_group_pets(
            GetGroupPetsInput(group_id="shelter", pet_ids=[1, 2])
        )
    ).status == 200
    assert isinstance(await client.health(HealthInput()), Health200)
    assert isinstance(
        await client.delete_pet(DeletePetInput(pet_id="p_123")), DeletePet204
    )
