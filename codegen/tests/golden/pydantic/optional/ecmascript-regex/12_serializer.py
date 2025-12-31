"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "[a-z]cole"
}

Tests:
[
  {
    "data": "Les hivers de mon enfance étaient des saisons longues, longues. Nous vivions en trois lieux: l'école, l'église et la patinoire; mais la vraie vie était sur la patinoire.",
    "description": "literal unicode character in json string",
    "valid": false
  },
  {
    "data": "Les hivers de mon enfance étaient des saisons longues, longues. Nous vivions en trois lieux: l'école, l'église et la patinoire; mais la vraie vie était sur la patinoire.",
    "description": "unicode character in hex format in string",
    "valid": false
  },
  {
    "data": "Les hivers de mon enfance etaient des saisons longues, longues. Nous vivions en trois lieux: l'ecole, l'eglise et la patinoire; mais la vraie vie etait sur la patinoire.",
    "description": "ascii characters match",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Ecmascriptregex12Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[str, Field(pattern="[a-z]cole")], config=ConfigDict(strict=True)): v if not isinstance(v, str) else _adapter.validate_python(v))]

