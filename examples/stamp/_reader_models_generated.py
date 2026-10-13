# Generated implementation. Import the public model module instead.
from __future__ import annotations

import typing
from jsoncompat.codegen import dataclasses as dc

__all__ = ("bind_models",)

def _jsoncompat_init_0(self: dc.DataclassModel, *, skip_validation: bool = False, root: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_init_1(self: dc.DataclassModel, *, skip_validation: bool = False, age: typing.Any, name: typing.Any, __jsoncompat_extra__: typing.Any = dc.EXTRA_DEFAULT) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_init_2(self: dc.DataclassModel, *, skip_validation: bool = False, age: typing.Any, interests: typing.Any, name: typing.Any, __jsoncompat_extra__: typing.Any = dc.EXTRA_DEFAULT) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_init_3(self: dc.DataclassModel, *, skip_validation: bool = False, version: typing.Any, data: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_init_4(self: dc.DataclassModel, *, skip_validation: bool = False, version: typing.Any, data: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_bind(_models: tuple[type[dc.DataclassModel], ...], _namespace: dict[str, typing.Any]) -> None:
    (UserProfileReader, UserProfileV1, UserProfileV2, UserProfileV2Reader, UserProfileV1Reader,) = _models
    dc.install_model(UserProfileReader, _jsoncompat_init_0, _namespace)
    dc.install_model(UserProfileV1, _jsoncompat_init_1, _namespace)
    dc.install_model(UserProfileV2, _jsoncompat_init_2, _namespace)
    dc.install_model(UserProfileV2Reader, _jsoncompat_init_3, _namespace)
    dc.install_model(UserProfileV1Reader, _jsoncompat_init_4, _namespace)
    dc.bind_module(1, [(UserProfileReader, 0), (UserProfileV1, 4), (UserProfileV2, 10), (UserProfileV2Reader, 8), (UserProfileV1Reader, 2)], [
        ('root', UserProfileReader, 1, b'{"version":4,"nodes":[{"types":null,"choices":null,"rules":[{"one":[1,9]}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["data",2],["version",7]],"patterns":[],"required":["version","data"],"additional":8}}]},{"types":null,"choices":null,"rules":[{"ref":{"node":3,"pointer":"/$defs/v2"}}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",4],["interests",5],["name",6]],"patterns":[],"required":["name","age","interests"],"additional":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":null,"choices":[2],"constant":true,"rules":[]},{"types":null,"choices":null,"rules":["false"]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["data",10],["version",12]],"patterns":[],"required":["version","data"],"additional":8}}]},{"types":null,"choices":null,"rules":[{"ref":{"node":11,"pointer":"/$defs/v1"}}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",4],["name",6]],"patterns":[],"required":["name","age"],"additional":null}}]},{"types":null,"choices":[1],"constant":true,"rules":[]}],"patterns":[],"pattern_sources":[]}'),
        ('union', (2, 8,), "version", ((1, 0), (2, 1),)),
        ('model', UserProfileV1Reader, (("data", "data", 4, False), ("version", "version", 3, False),), None, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["data",1],["version",5]],"patterns":[],"required":["version","data"],"additional":6}}]},{"types":null,"choices":null,"rules":[{"ref":{"node":2,"pointer":"/$defs/v1"}}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",3],["name",4]],"patterns":[],"required":["name","age"],"additional":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":null,"choices":[1],"constant":true,"rules":[]},{"types":null,"choices":null,"rules":["false"]}],"patterns":[],"pattern_sources":[]}'),
        ('literal', (1,)),
        ('model', UserProfileV1, (("age", "age", 5, False), ("name", "name", 6, False),), 7, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",1],["name",2]],"patterns":[],"required":["age","name"],"additional":null}},{"object_length":{"min":2,"max":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}],"patterns":[],"pattern_sources":[]}'),
        ("int",),
        ("str",),
        ("any",),
        ('model', UserProfileV2Reader, (("data", "data", 10, False), ("version", "version", 9, False),), None, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["data",1],["version",6]],"patterns":[],"required":["version","data"],"additional":7}}]},{"types":null,"choices":null,"rules":[{"ref":{"node":2,"pointer":"/$defs/v2"}}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",3],["interests",4],["name",5]],"patterns":[],"required":["name","age","interests"],"additional":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":null,"choices":[2],"constant":true,"rules":[]},{"types":null,"choices":null,"rules":["false"]}],"patterns":[],"pattern_sources":[]}'),
        ('literal', (2,)),
        ('model', UserProfileV2, (("age", "age", 5, False), ("interests", "interests", 5, False), ("name", "name", 6, False),), 7, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",1],["interests",2],["name",3]],"patterns":[],"required":["age","interests","name"],"additional":null}},{"object_length":{"min":3,"max":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}],"patterns":[],"pattern_sources":[]}')
    ], b'{"version":4,"base_nodes":11,"guard_nodes":[{"original":5,"guard":{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]}},{"original":6,"guard":{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}}],"guards":[{"owner":4,"field":0,"guard":0},{"owner":4,"field":1,"guard":1},{"owner":10,"field":0,"guard":0},{"owner":10,"field":2,"guard":1}],"conversion_validates":[true,false,true,false,true,false,false,false,true,false,true,false,false],"json_keys":[[2,["\\"data\\":","\\"version\\":"]],[4,["\\"age\\":","\\"name\\":"]],[8,["\\"data\\":","\\"version\\":"]],[10,["\\"age\\":","\\"interests\\":","\\"name\\":"]]]}')


bind_models = _jsoncompat_bind
