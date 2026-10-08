# Generated implementation. Import the public model module instead.
from __future__ import annotations

import typing
from jsoncompat.codegen import dataclasses as dc

__all__ = ("bind_models",)

def _jsoncompat_init_0(self: dc.DataclassModel, *, skip_validation: bool = False, version: typing.Any, data: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_init_1(self: dc.DataclassModel, *, skip_validation: bool = False, age: typing.Any, interests: typing.Any, name: typing.Any, __jsoncompat_extra__: typing.Any = dc.EXTRA_DEFAULT) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_bind(_models: tuple[type[dc.DataclassModel], ...], _namespace: dict[str, typing.Any]) -> None:
    (UserProfileWriter, UserProfileV2,) = _models
    dc.install_model(UserProfileWriter, _jsoncompat_init_0, "{\n  \"$defs\": {\n    \"v2\": {\n      \"properties\": {\n        \"age\": {\n          \"minimum\": 0,\n          \"type\": \"integer\"\n        },\n        \"interests\": {\n          \"type\": \"integer\"\n        },\n        \"name\": {\n          \"minLength\": 1,\n          \"type\": \"string\"\n        }\n      },\n      \"required\": [\n        \"name\",\n        \"age\",\n        \"interests\"\n      ],\n      \"type\": \"object\",\n      \"x-jsoncompat\": {\n        \"kind\": \"declaration\",\n        \"name\": \"UserProfileV2\",\n        \"schema_ref\": \"#/$defs/v2\",\n        \"stable_id\": \"user-profile\",\n        \"version\": 2\n      }\n    }\n  },\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n  \"additionalProperties\": false,\n  \"properties\": {\n    \"data\": {\n      \"$ref\": \"#/$defs/v2\"\n    },\n    \"version\": {\n      \"const\": 2\n    }\n  },\n  \"required\": [\n    \"version\",\n    \"data\"\n  ],\n  \"title\": \"user-profile writer v2\",\n  \"type\": \"object\",\n  \"x-jsoncompat\": {\n    \"kind\": \"writer\",\n    \"name\": \"UserProfileWriter\",\n    \"payload_ref\": \"#/$defs/v2\",\n    \"stable_id\": \"user-profile\",\n    \"version\": 2\n  }\n}", b'{"version":1,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["data",1],["version",6]],"patterns":[],"required":["version","data"],"additional":7}}]},{"types":null,"choices":null,"rules":[{"ref":2}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",3],["interests",4],["name",5]],"patterns":[],"required":["name","age","interests"],"additional":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":null,"choices":[2],"rules":[]},{"types":null,"choices":null,"rules":["false"]}],"patterns":[]}', _namespace)
    dc.install_model(UserProfileV2, _jsoncompat_init_1, "{\n  \"properties\": {\n    \"age\": {\n      \"minimum\": 0,\n      \"type\": \"integer\"\n    },\n    \"interests\": {\n      \"type\": \"integer\"\n    },\n    \"name\": {\n      \"minLength\": 1,\n      \"type\": \"string\"\n    }\n  },\n  \"required\": [\n    \"name\",\n    \"age\",\n    \"interests\"\n  ],\n  \"type\": \"object\",\n  \"x-jsoncompat\": {\n    \"kind\": \"declaration\",\n    \"name\": \"UserProfileV2\",\n    \"schema_ref\": \"#/$defs/v2\",\n    \"stable_id\": \"user-profile\",\n    \"version\": 2\n  }\n}", b'{"version":1,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",1],["interests",2],["name",3]],"patterns":[],"required":["name","age","interests"],"additional":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}],"patterns":[]}', _namespace)
    dc.bind_module(1, [(UserProfileWriter, 0), (UserProfileV2, 2)], [
        ('model', UserProfileWriter, (("data", "data", 2, False), ("version", "version", 1, False),), None),
        ('literal', (2,)),
        ('model', UserProfileV2, (("age", "age", 3, False), ("interests", "interests", 3, False), ("name", "name", 4, False),), 5),
        ("int",),
        ("str",),
        ("any",)
    ], b'{"version":1,"base_nodes":6,"guards":[{"owner":2,"field":0,"original":3,"guard":{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]}},{"owner":2,"field":2,"original":4,"guard":{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}}],"conversion_validates":[true,false,true,false,false,false,false,false],"json_keys":[[0,["\\"data\\":","\\"version\\":"]],[2,["\\"age\\":","\\"interests\\":","\\"name\\":"]]]}')


bind_models = _jsoncompat_bind
