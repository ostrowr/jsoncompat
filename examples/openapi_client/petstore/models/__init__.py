"""Generated model package. Import a model by name; unrelated modules stay unloaded."""
from importlib import import_module as _import_module
from .._exports import MODELS as _MODELS

def __getattr__(name: str):
    module = _MODELS.get(name)
    if module is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    value = getattr(_import_module(f"..{module}", __package__), name)
    globals()[name] = value
    return value

def __dir__():
    return sorted(set(globals()) | _MODELS.keys())

__all__: tuple[str, ...] = (
    "CreatePet201",
    "CreatePet201Headers",
    "CreatePet400",
    "CreatePet400Headers",
    "CreatePetDefault",
    "CreatePetDefaultHeaders",
    "CreatePetRequest",
    "DeletePet204",
    "DeletePet204Headers",
    "DeletePet404",
    "DeletePet404Headers",
    "DeletePetRequest",
    "GetGroupPets200",
    "GetGroupPets200Headers",
    "GetGroupPetsRequest",
    "GetPet200",
    "GetPet200Headers",
    "GetPet304",
    "GetPet304Headers",
    "GetPet404",
    "GetPet404Headers",
    "GetPetNote200ApplicationJson",
    "GetPetNote200Headers",
    "GetPetNote200TextPlain",
    "GetPetNote404",
    "GetPetNote404Headers",
    "GetPetNoteRequest",
    "GetPetRequest",
    "Health",
    "Health200",
    "Health200Headers",
    "Health2XX",
    "Health2XXHeaders",
    "Health5XX",
    "Health5XXHeaders",
    "HealthDefault",
    "HealthDefaultHeaders",
    "HealthRequest",
    "JsoncompatEmptyHeaders",
    "ListPets200",
    "ListPets200Headers",
    "ListPets400",
    "ListPets400Headers",
    "ListPetsDefault",
    "ListPetsDefaultHeaders",
    "ListPetsRequest",
    "Pet",
    "PetCreate",
    "PetFilter",
    "PetPage",
    "PetPatch",
    "Problem",
    "SearchCriteria",
    "SearchPets200",
    "SearchPets200Headers",
    "SearchPets400",
    "SearchPets400Headers",
    "SearchPetsRequest",
    "SetPetNote200",
    "SetPetNote200Headers",
    "SetPetNote404",
    "SetPetNote404Headers",
    "SetPetNoteBodyApplicationJson",
    "SetPetNoteBodyTextPlain",
    "SetPetNoteRequest",
    "UpdatePet200",
    "UpdatePet200Headers",
    "UpdatePet404",
    "UpdatePet404Headers",
    "UpdatePet412",
    "UpdatePet412Headers",
    "UpdatePetRequest",
)
