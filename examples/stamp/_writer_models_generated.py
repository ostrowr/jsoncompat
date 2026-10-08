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
    dc.install_model(UserProfileV2, _jsoncompat_init_1, b'x\xda}\x90\xcdn\xc2@\x0c\x84\xefy\x8a\xc8\xed\x11D\xcb\x91g\xe8\xa1\x97\xf6RUhI\x9c`\x9a\xfd\xa9\xed\xa0V\x88wgw!\x84\x1f\x89\xa3\xc7\xe3ovvW\x94%\x04\xf6\x01Y\x09\x05\x16\xe5.*Q3-\x9e\x878Zrd{\x1b\xa5\x97\xc9\xa0\xe9\x7fH\x1e \xa7\xd8"C\xd6\xf7\xc7u\x16\x19E\xe5\x92\xf2\xf0\xc2\x19{\x1b\xf9\x86\xae\xd5u\x14_\xefBE\x99\\{"\x14\'\x0a0\xfe\xf6\xc4XG\xc7\xd7\x05u2v\xba{^\x9c\xbf\xf3\xed@\xf6\xab\x0dV\x9a\x8d\xf07\xdd\x88w\x95\xb7\xc1\xe8\xf87?\xe4R\x02\xd4Xu\x86\x8d\x92wp]\x02>\x04\xf9\x9d}C\x1d~\xce\x87\xa5Tk\xb4f\xc9\xd8$\xcb\xd3\xec\xb9\xc6Ff\xdbq\xaff\xd5\xe1\x922\xbc\x8f\x84i8"\x06\xc3\x16YR\xd8\xa2\x9c\xa7\xca\xc5\xfe\x00\xbd\x8bv\x1e', _namespace)
    dc.bind_module(1, [(UserProfileWriter, 0), (UserProfileV2, 2)], [
        ('model', UserProfileWriter, (("data", "data", 2, False), ("version", "version", 1, False),), None, b'{"version":1,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["data",1],["version",6]],"patterns":[],"required":["version","data"],"additional":7}}]},{"types":null,"choices":null,"rules":[{"ref":2}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",3],["interests",4],["name",5]],"patterns":[],"required":["name","age","interests"],"additional":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":null,"choices":[2],"rules":[]},{"types":null,"choices":null,"rules":["false"]}],"patterns":[]}'),
        ('literal', (2,)),
        ('model', UserProfileV2, (("age", "age", 3, False), ("interests", "interests", 3, False), ("name", "name", 4, False),), 5, b'{"version":1,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["age",1],["interests",2],["name",3]],"patterns":[],"required":["name","age","interests"],"additional":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}],"patterns":[]}'),
        ("int",),
        ("str",),
        ("any",)
    ], b'{"version":2,"base_nodes":6,"guard_nodes":[{"original":3,"guard":{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]}},{"original":4,"guard":{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}}],"guards":[{"owner":2,"field":0,"guard":0},{"owner":2,"field":2,"guard":1}],"conversion_validates":[true,false,true,false,false,false,false,false],"json_keys":[[0,["\\"data\\":","\\"version\\":"]],[2,["\\"age\\":","\\"interests\\":","\\"name\\":"]]]}')


bind_models = _jsoncompat_bind
