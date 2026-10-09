# Generated implementation. Import the public model module instead.
from __future__ import annotations

import typing
from jsoncompat.codegen import dataclasses as dc

__all__ = ("bind_models",)

_jsoncompat_missing = dc.JsoncompatMissingType()

def _jsoncompat_init_0(self: dc.DataclassModel, *, skip_validation: bool = False, customer: typing.Any, id: typing.Any, items: typing.Any, note: typing.Any = _jsoncompat_missing, status: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_init_1(self: dc.DataclassModel, *, skip_validation: bool = False, email: typing.Any, name: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_init_2(self: dc.DataclassModel, *, skip_validation: bool = False, quantity: typing.Any, sku: typing.Any, unitPriceCents: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_bind(_models: tuple[type[dc.DataclassModel], ...], _namespace: dict[str, typing.Any]) -> None:
    (Order, Customer, OrderItem,) = _models
    dc.install_model(Order, _jsoncompat_init_0, b'x\xda\xadT\xbbn\xc20\x14\xdd\xf3\x15\xc8e\x0c\x0d\xd0\x8d\x95\x09\xa9R\xd9\xab\x0en|\x13L\x13\'\xd8\xd7\x03B\xfc{\x9d\'v\xe2\x00\x03\xa3\xef\xf3\xf8\x9cc_\x82\xd9\x8c\xcc\x19$\x8alf\x17s0\xc7X+,r\x90}\xc4\xc4(c\x1cy!h\xb6\x97E\x09\x129T\x1d\x09\xcd\x14\x84]Qi\xa7\xbaV\x13\x87\x9c\xf2\xcc\x09\x99`\xce\xc5\'\x88\x14\x0f&\xf1\x11\xda\x19<\x97`\x82D\xa1\xe4"%}\xeaz\xab"\x82\xe6po\xe2\xea\xb9\x89\xc1`2\x91p\xd2\\\x023\xd5\xdf\x83m\xe1\xf0B\xed\xf9\xa7oF\x8eY\xbdg\xdb1xK\xb5\x08\x8a\xdf#\xc4\xd8\xb4\xb6[\x09G\xc8_\xc6\xf5ISap\x9c=\xe4\xf0\\\xe7\x93\xd4p\x81\x90\x1a\xc4^\xb6\xd5\x9f~\x01\xd9\xd6@-8\xee%\x8fa\x0b\x02\xd54\xd6\xe5\x93X\x9f\xd4\xb1\xbaG\xe8\xe1j\x1a\xd9\xb4\xc6_\x92\x81\xdcU\xd2\xdd\x179ha\x91\xb9\x8a\x0f\xc68U\xc1\x01\xb1T\x9b(:\xaaB,\x9a\xf0{!\xd3\x88I\x9a`\xb4^\xae\x97\x8b\xd5:j\xeb\xeb\xe6G\x9e\xf0\xf9\xc1\xfb\x90\xe7\x12\x92\x0a\xc2[T\xbf\xfa\xa8\xafq-\xc9\xec\x1e\xaf\xd6^\x9dmK\xdb\xb2\x8e\x02\x1e$\xf53\x18\x09iv\xef\xda\xde\xf1j*%=\xbb\x9bE\x81\xf6\xc7\xd0\x95\xda&h\xf0\xda\x9f\x89\xcen\xaf\xd9\x99\xa6\x90\xa2v.\x02\xa26\xa65\xaf\x04\xc1\x06\x03Kj\x08t\x07\xf66\x189\xb3";\x1c\x08\xe6\xd0\xe8b\x09Z7\x0e\x9c\xd8\xd8d\xe8\xc0\xeb?\xb3\x0dp#', _namespace)
    dc.install_model(Customer, _jsoncompat_init_1, b'x\xda}\xce\xbb\x12\xc2 \x10\x05\xd0\x9e\xaf`\xb6N\xa3v\xb6\xb6\x16\xf6\x8e\x05\xca\x1a\xd7\xe1%l\x0a\'\x93\x7f\x17!:X\x98\x0e\xce^\xee2\x0a)AiML\xde)s\x88>`d\xc2\x04[yU&a\xf7\x0eXr?\x93u\xd1\xd0\xd2\x98%\x1bZE\xe6{\xadO\xf7\xe8z\xbee\xdct\x1f\xe5g\xc0\x0c\x908\x92\xeb\xa1\xf0T\xa7\xe0\x94\xc5\x7f\x0d\xab\xe5\x061\xb7@\xc4\xc7@\x11uN\x1c\xdb\x8f\xb5+\xf2\xf1T\xc2LlJ\xd7nH\xec-F\xa8<o\xf0\xe7;^\x18\xc4\xf4\x02T\x17P\xe6', _namespace)
    dc.install_model(OrderItem, _jsoncompat_init_2, b'x\xda\x9d\xd0\xb1\x0e\x820\x10\x06\xe0\x9d\xa7 \x9d\x194n\xaeN&&\xb0\x1b\x07\x84\x03O\xdbR\xae\xd7\x81\x10\xde\xddZ\x08\x01\xc2\xe4\xfa\xdd\xf5\xbf\xeb\xf5Q\x1c\x8b\xbc,\x91\xb1\xd1\xb9\xcc\xa81@\x8c`\xc59\xaeri!\xf95(\xd4\xab\xca)\xa8YR\xef\xc5[\xebr\xcd\xc8\xdd,\xe3kTNy:&\xb39\xc9h$\xa4\xd5\x8a\xb93\xe0A\xa0f\xa8\x81D\xf0a,\x0b\xfbq\x9b\xd8\x1b\xe8\x9a_\xbb\x09\x96\x09u\xbd\x0ep\x1a9#,\xe0\x02\x9a\xed\xfe\x8a\x87\xffV\x8c\xa6)\x82\xa0uHP\xfa\x96\xfb\xe6$\x8bo\xec/\xe4\xf1\x11B|\xbb\x0cCR*\x81\xae\x0cJ\x8c>\xcdn\x9eo(XD\xc3\x17\x0b\x88y5', _namespace)
    dc.bind_module(1, [(Order, 0), (Customer, 1), (OrderItem, 4)], [
        ('model', Order, (("customer", "customer", 1, False), ("id", "id", 2, False), ("items", "items", 3, False), ("note", "note", 6, True), ("status", "status", 8, False),), None, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["customer",1],["id",4],["items",6],["note",11],["status",12]],"patterns":[],"required":["id","customer","items","status"],"additional":5}}]},{"types":null,"choices":null,"rules":[{"ref":{"node":2,"pointer":"/$defs/customer"}}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["email",3],["name",4]],"patterns":[],"required":["name","email"],"additional":5}}]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":3,"max":null}}]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":null,"choices":null,"rules":["false"]},{"types":["array"],"choices":null,"rules":[{"array":{"prefix":[],"items":7}},{"array_length":{"min":1,"max":null}}]},{"types":null,"choices":null,"rules":[{"ref":{"node":8,"pointer":"/$defs/item"}}]},{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["quantity",9],["sku",4],["unitPriceCents",10]],"patterns":[],"required":["sku","quantity","unitPriceCents"],"additional":5}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":1,"lower":true,"exclusive":false}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":["string","null"],"choices":null,"rules":[]},{"types":null,"choices":["pending","paid"],"rules":[]}],"patterns":[],"pattern_sources":[]}'),
        ('model', Customer, (("email", "email", 2, False), ("name", "name", 2, False),), None, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["email",1],["name",2]],"patterns":[],"required":["email","name"],"additional":3}},{"object_length":{"min":2,"max":null}}]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":3,"max":null}}]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":null,"choices":null,"rules":["false"]}],"patterns":[],"pattern_sources":[]}'),
        ("str",),
        ('list', 4),
        ('model', OrderItem, (("quantity", "quantity", 5, False), ("sku", "sku", 2, False), ("unitPriceCents", "unitPriceCents", 5, False),), None, b'{"version":4,"nodes":[{"types":["object"],"choices":null,"rules":[{"object":{"properties":[["quantity",1],["sku",2],["unitPriceCents",3]],"patterns":[],"required":["quantity","sku","unitPriceCents"],"additional":4}},{"object_length":{"min":3,"max":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":1,"lower":true,"exclusive":false}}]},{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]},{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]},{"types":null,"choices":null,"rules":["false"]}],"patterns":[],"pattern_sources":[]}'),
        ("int",),
        ('union', (2, 7,), None, None),
        ("null",),
        ('literal', ("paid", "pending",)),
        ('literal', ("paid",)),
        ('literal', ("pending",))
    ], b'{"version":4,"base_nodes":11,"guard_nodes":[{"original":2,"guard":{"types":["string"],"choices":null,"rules":[{"string_length":{"min":1,"max":null}}]}},{"original":2,"guard":{"types":["string"],"choices":null,"rules":[{"string_length":{"min":3,"max":null}}]}},{"original":5,"guard":{"types":["integer"],"choices":null,"rules":[{"bound":{"value":1,"lower":true,"exclusive":false}}]}},{"original":5,"guard":{"types":["integer"],"choices":null,"rules":[{"bound":{"value":0,"lower":true,"exclusive":false}}]}}],"guards":[{"owner":0,"field":1,"guard":0},{"owner":1,"field":0,"guard":1},{"owner":1,"field":1,"guard":0},{"owner":4,"field":0,"guard":2},{"owner":4,"field":1,"guard":0},{"owner":4,"field":2,"guard":3}],"conversion_validates":[false,true,false,false,true,false,false,false,false,false,false,false,false,false,false],"json_keys":[[0,["\\"customer\\":","\\"id\\":","\\"items\\":","\\"note\\":","\\"status\\":"]],[1,["\\"email\\":","\\"name\\":"]],[4,["\\"quantity\\":","\\"sku\\":","\\"unitPriceCents\\":"]]]}')


bind_models = _jsoncompat_bind
