# Generated implementation. Import the public model module instead.
from __future__ import annotations

import typing
from jsoncompat.codegen import dataclasses as dc

__all__ = ("bind_models",)

_jsoncompat_missing = dc.MissingType.MISSING

def _jsoncompat_init_0(self: dc.DataclassModel, *, skip_validation: bool = False, customer: typing.Any, id: typing.Any, items: typing.Any, note: typing.Any = _jsoncompat_missing, status: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_init_1(self: dc.DataclassModel, *, skip_validation: bool = False, email: typing.Any, name: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_init_2(self: dc.DataclassModel, *, skip_validation: bool = False, quantity: typing.Any, sku: typing.Any, unitPriceCents: typing.Any) -> None:
    self.__post_init__(skip_validation)

def _jsoncompat_bind(_models: tuple[type[dc.DataclassModel], ...], _namespace: dict[str, typing.Any]) -> None:
    (Order, Customer, OrderItem,) = _models
    dc.install_model(Order, _jsoncompat_init_0, _namespace)
    dc.install_model(Customer, _jsoncompat_init_1, _namespace)
    dc.install_model(OrderItem, _jsoncompat_init_2, _namespace)
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
