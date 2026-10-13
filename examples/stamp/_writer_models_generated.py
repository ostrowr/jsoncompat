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
    dc.install_model(UserProfileWriter, _jsoncompat_init_0, _namespace)
    dc.install_model(UserProfileV2, _jsoncompat_init_1, _namespace)
    dc.bind_module(1, [(UserProfileWriter, 0), (UserProfileV2, 2)], [
        ('model', UserProfileWriter, (("data", "data", 2, False), ("version", "version", 1, False),), None, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["data",1],["version",6]],"patterns":[],"required":["version","data"],"additional":7}}]},{"types":null,"choices":null,"rules":[{"ref":{"node":2,"pointer":"/$defs/v2"}}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",3],["interests",4],["name",5]],"patterns":[],"required":["name","age","interests"],"additional":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":null,"choices":[2],"constant":true,"rules":[]},{"types":null,"choices":null,"rules":["false"]}],"patterns":[],"pattern_sources":[]}'),
        ('literal', (2,)),
        ('model', UserProfileV2, (("age", "age", 3, False), ("interests", "interests", 3, False), ("name", "name", 4, False),), 5, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",1],["interests",2],["name",3]],"patterns":[],"required":["age","interests","name"],"additional":null}},{"object_length":{"min":3,"max":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}],"patterns":[],"pattern_sources":[]}'),
        ("int",),
        ("str",),
        ("any",)
    ], b'{"version":4,"base_nodes":6,"guard_nodes":[{"original":3,"guard":{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]}},{"original":4,"guard":{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}}],"guards":[{"owner":2,"field":0,"guard":0},{"owner":2,"field":2,"guard":1}],"conversion_validates":[true,false,true,false,false,false,false,false],"json_keys":[[0,["\\"data\\":","\\"version\\":"]],[2,["\\"age\\":","\\"interests\\":","\\"name\\":"]]]}')


bind_models = _jsoncompat_bind
