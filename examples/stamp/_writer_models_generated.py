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
    dc.install_model(UserProfileWriter, _jsoncompat_init_0, b'x\xda\x8dR\xc1r\x820\x10\xbd\xf3\x15L\xeaQ\x8ar\xf4\x1bz\xf0\xd2\xf6\xd0\xe98\x91,\x18\x0b\x09\xdd\xac\xb6\x8e\xe3\xbf7\x01\x8c\x01\xe9\xb4\x17\x86l\xde\xbe\x97\xb7o\xcfQ\x1c\xb3\x99\x80\xc2\xb0U|\xb6\x07{<f\xfe\xdf\x9e\x1a\xd4\x0d I0A\xd5\xd6y\x09\x83\x82-\xd5R\xc9\xfaP\xdb\xf2b\x1e\xd6\xe9\xd48,\x93\x8a\xa0\x04d\xfe\xeer\x83\xb5\x97\x08\x86\xcc\x98\xf5_\xdd\x8a\xd7S\xcfy\x02U\xd2\xce^,\'\x1fd\x08\xa5*\x03\xc6h\xc4\xcc\x10>\x0f\x12AX\xf4\xdbHm>\x9c\xc5\xa4\x95\xbe\xf6\xee\xf9\xae\xcaz\xbb\x87\x9c|\x13\xfbN\xf6F\xab\\\xd7\x0d\xa7\xe1\x9c?\xa4r\xeaL@^q\xe4$\xb5b\xf7\xc6\xd9\xb3\x01\\\xa3.d\x05/Y\x080\xf9\x0ej\xbeA(\x1c\xec!m\xc3N\x8fC\x0c\xf1m\x05\x1b\xd9\x0a\x1d,S\xd2tT!\xe8\x08h\x9c\xf8*\xce\xa2p\\\xee\xdb\x0e\x8c\xcd:-G\xb2#j\xcc*M\x9d\xab\xa4+?j,S\x81\xbc\xa04[d\x8bd\x99\xa5=\xbem\xe6BHg\x8eW\xebp\xe3\x0a^\x19h\x01\x13\x8b\xc8\x04\'\x1e.\xeb\xec\xdef\x14\x04\x1ax\xf0-\xb9V\x86\xbc\xa9\x9b\x99\xbb\xe4}\xf3<\xd0\x8e\xfap\x19I\xaa`<\xbe\xf8\x0b\xa5]\x85\xb8\x9f\xf6T\xfa\xd3\xc9\xfb\xd4;\x82\xab\xe4D\xd8\xaf\x03@\xc3O\x95\xe6\xe2\xd7\xb4\xffLz\x94\xf2%\xba\xfc\x00\xd3\x10\x0fL', _namespace)
    dc.install_model(UserProfileV2, _jsoncompat_init_1, b'x\xda\x9d\x91\xcdN\xc30\x0c\x80\xef}\x8a\xcap\xdc4\x18\xb7=\x03\x12\\\xe0\x82\xd0\x94\xb5n\xe7\x91?\x1cw\x02M{w\x92tm7\xe0\xb4[\xf2\xd9\xfe\x1c;\x87\xa2,\xc1\x90}f\xe7\x91\x850\xc0\xaa|\x98%\xea\xcf\xd1!\x92\xc8T\x8b\xe3\xa5/$\xd3\x99\x88\xeef#\xeb\xb4\x90\xd7\xf8\xd4D|?b\xf9\xf6\xa9\x14\xc8\x0a\xb6\xc8\x90\xf9\xb1\x0fg\xc8\x18$\\\xc8\xaf\x10Ye~?\xf0\x11m+\xdb\x7f\x15A\x98l{2\x14\'\x0b0~v\xc4X\xc7\x8c\xb7i\xea?/=\xef\x18\x8f\xef\xb9v0\xbb\xcd\x0e+\xc99\xf05\xdf\x05g+g\xbc\x92i\x93\x1fdS\x07\xa8\xb1\xd2\x8a\x95\x90\xb3p9\x04\xbc\x04\xe4\xf8/\x0di|]\x0e\xc1Pm\xd1\xa85c\xda\x0a\xdc,nkl\xc2b?\xc5Em4\xae)\xcb\xbbh\x98\xfb^1$\xec\x91Cj\xb6*\x97i\xe4\xe2\xf8\x03\xc4u\x88\xa2', _namespace)
    dc.bind_module(1, [(UserProfileWriter, 0), (UserProfileV2, 2)], [
        ('model', UserProfileWriter, (("data", "data", 2, False), ("version", "version", 1, False),), None, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["data",1],["version",6]],"patterns":[],"required":["version","data"],"additional":7}}]},{"types":null,"choices":null,"rules":[{"ref":{"node":2,"pointer":"/$defs/v2"}}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",3],["interests",4],["name",5]],"patterns":[],"required":["name","age","interests"],"additional":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":null,"choices":[2],"constant":true,"rules":[]},{"types":null,"choices":null,"rules":["false"]}],"patterns":[],"pattern_sources":[]}'),
        ('literal', (2,)),
        ('model', UserProfileV2, (("age", "age", 3, False), ("interests", "interests", 3, False), ("name", "name", 4, False),), 5, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",1],["interests",2],["name",3]],"patterns":[],"required":["age","interests","name"],"additional":null}},{"object_length":{"min":3,"max":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}],"patterns":[],"pattern_sources":[]}'),
        ("int",),
        ("str",),
        ("any",)
    ], b'{"version":4,"base_nodes":6,"guard_nodes":[{"original":3,"guard":{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]}},{"original":4,"guard":{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}}],"guards":[{"owner":2,"field":0,"guard":0},{"owner":2,"field":2,"guard":1}],"conversion_validates":[true,false,true,false,false,false,false,false],"json_keys":[[0,["\\"data\\":","\\"version\\":"]],[2,["\\"age\\":","\\"interests\\":","\\"name\\":"]]]}')


bind_models = _jsoncompat_bind
