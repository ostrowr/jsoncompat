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

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "[a-z]cole"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex12Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

