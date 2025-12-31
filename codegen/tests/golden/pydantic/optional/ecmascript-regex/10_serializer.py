"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "\\p{Letter}cole"
}

Tests:
[
  {
    "data": "Les hivers de mon enfance etaient des saisons longues, longues. Nous vivions en trois lieux: l'ecole, l'eglise et la patinoire; mais la vraie vie etait sur la patinoire.",
    "description": "ascii character in json string",
    "valid": true
  },
  {
    "data": "Les hivers de mon enfance étaient des saisons longues, longues. Nous vivions en trois lieux: l'école, l'église et la patinoire; mais la vraie vie était sur la patinoire.",
    "description": "literal unicode character in json string",
    "valid": true
  },
  {
    "data": "Les hivers de mon enfance étaient des saisons longues, longues. Nous vivions en trois lieux: l'école, l'église et la patinoire; mais la vraie vie était sur la patinoire.",
    "description": "unicode character in hex format in string",
    "valid": true
  },
  {
    "data": "LES HIVERS DE MON ENFANCE ÉTAIENT DES SAISONS LONGUES, LONGUES. NOUS VIVIONS EN TROIS LIEUX: L'ÉCOLE, L'ÉGLISE ET LA PATINOIRE; MAIS LA VRAIE VIE ÉTAIT SUR LA PATINOIRE.",
    "description": "unicode matching is case-sensitive",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "\\p{Letter}cole"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex10Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

