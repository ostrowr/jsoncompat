"""Reviewable witnesses plus a deterministic schema-composition matrix."""

import itertools
import json
import math


def record(properties, required=None, **keywords):
    return {"type": "object", "properties": properties,
            "required": list(properties) if required is None else required,
            "additionalProperties": False, **keywords}


def cases():
    result = []

    def add(name, schema, valid, invalid, *, inline=None, wires=(), stress=False):
        result.append(dict(name=name, schema=schema, valid=valid, invalid=invalid,
                           inline=inline, wires=list(wires), stress=stress))

    add("control", record({"value": {"type": "integer", "minimum": 0}}),
        [{"value": 0}, {"value": 1}], [{"value": -1}, {"value": True}, {}], inline=True, wires=[
        ['{"value":1.000000000000000000000000000000001}', False],
        ['{"value":1.0}', True], ['{"value":1e-400}', False]])
    scalars = [
        ("integer", {"type": "integer", "minimum": -2, "exclusiveMaximum": 3}, [-2, 0, 2, 2.0], [-3, 3, 1.5, True]),
        ("number", {"type": "number", "exclusiveMinimum": -1, "maximum": 1},
         [0, -0.0, 1, math.nextafter(1, 0)], [-1, math.nextafter(1, 2), True]),
        ("multiple", {"type": "number", "multipleOf": 0.25}, [-1, 0, 0.25, 1.5], [0.1, 0.3, True]),
        ("decimal_multiple", {"type": "number", "multipleOf": 0.1}, [-0.3, 0, 0.3, 1], [0.30000000000000004, -0.30000000000000004, 0.01, True]),
        ("integer_multiple", {"type": "integer", "multipleOf": 3}, [-(2**64) + 1, 0, 3, 2**64 - 1], [2**64, 2**64 + 1, True]),
        ("unicode", {"type": "string", "minLength": 2, "maxLength": 3},
         ["ab", "é猫", "🐲🐲", "e\u0301", "a\x00b"], ["", "🐲", "abcd", 2]),
        ("enum", {"enum": [False, 0, 2, "x", None]}, [False, 0, 2.0, "x", None], [True, 1, "0", [], {}]),
        ("const_bool", {"const": True}, [True], [False, 1, 1.0, "true"]),
        ("const_number", {"const": 1}, [1, 1.0], [True, False, 0, "1"]),
        ("huge_integer", {"type": "integer", "minimum": 2**53, "maximum": 2**64},
         [2**53, 2**53 + 1, 2**63, 2**64], [2**53 - 1, 2**64 + 1, True]),
        ("huge_const", {"const": 2**64}, [2**64], [2**64 - 1, 2**64 + 1, True]),
        ("negative_integer", {"type": "integer", "minimum": -(2**64), "maximum": -(2**53)},
         [-(2**64), -(2**63), -(2**53) - 1], [-(2**64) - 1, -(2**53) + 1]),
        ("nullable", {"type": ["string", "null"], "minLength": 1}, [None, "x", "猫"], ["", 0, False]),
        ("pattern", {"type": "string", "pattern": r"\bcat\b"}, ["cat", " cat!", "🐲cat🐲"], ["catfish", "écat", ""]),
    ]
    for name, schema, valid, invalid in scalars:
        add(name, schema, valid, invalid)
        add(name + "_field", record({"x": schema}), [{"x": v} for v in valid], [{"x": v} for v in invalid] + [{}, {"x": valid[0], "extra": 0}])
        add(name + "_optional", record({"x": schema}, required=[]), [{}] + [{"x": v} for v in valid], [{"x": v} for v in invalid])
        add(name + "_array", {"type": "array", "items": schema}, [[], valid], [[v] for v in invalid])
        add(name + "_ref", {"$defs": {"a/b~c": schema}, "$ref": "#/$defs/a~1b~0c"}, valid, invalid)
        add(name + "_allof", {"allOf": [schema, schema]}, valid, invalid)
        add(name + "_anyof_duplicate", {"anyOf": [schema, schema]}, valid, invalid)
        add(name + "_oneof_duplicate", {"oneOf": [schema, schema]}, [], valid + invalid)
        add(name + "_not", {"not": schema}, invalid, valid)

    # Cross keyword placement with field presence. A missing field is not null,
    # and constraints in sibling applicators still apply to the same property.
    for op, optional, closed in itertools.product(("allOf", "anyOf", "oneOf"), (False, True), (False, True)):
        a = {"type": "object", "properties": {"x": {"type": "integer", "minimum": 0}}}
        b = {"type": "object", "properties": {"x": {"type": "integer", "maximum": 2}}}
        schema = {op: [a, b], "type": "object", "properties": {"x": {}},
                  "required": [] if optional else ["x"], "additionalProperties": not closed}
        candidates = [{}, {"x": None}, {"x": False}, {"x": -1}, {"x": 0}, {"x": 2}, {"x": 3}, {"x": 1, "z": 2}]
        valid, invalid = [], []
        for value in candidates:
            x = value.get("x")
            matches = 2 if "x" not in value else (int(type(x) is int and x >= 0) + int(type(x) is int and x <= 2))
            accepted = {"allOf": matches == 2, "anyOf": matches >= 1, "oneOf": matches == 1}[op]
            accepted &= (optional or "x" in value) and (not closed or "z" not in value)
            (valid if accepted else invalid).append(value)
        add(f"siblings_{op}_{optional}_{closed}", schema, valid, invalid)

    add("required_property_count", record({"x": {"type": "integer"}, "y": {"type": "string"}}, required=["x"], minProperties=1, maxProperties=2),
        [{"x": 1}, {"x": 1, "y": "ok"}], [{}, {"x": 1, "y": "ok", "z": 0}], inline=True)
    add("optional_property_count", record({"x": {"type": "integer"}, "y": {"type": "string"}}, required=[], minProperties=1, maxProperties=1),
        [{"x": 1}, {"y": "ok"}], [{}, {"x": 1, "y": "ok"}], inline=False)
    left = record({"kind": {"const": "left"}, "v": {"type": "integer", "minimum": 0}})
    right = record({"kind": {"const": "right"}, "v": {"type": "string", "minLength": 1}})
    add("tagged_union", {"oneOf": [left, right]}, [{"kind": "left", "v": 0}, {"kind": "right", "v": "x"}],
        [{"kind": "left", "v": -1}, {"kind": "left", "v": "x"}, {"kind": "right", "v": ""}, {"v": 1}])
    add("union_guard_backtracking", {"anyOf": [record({"bar": {"type": "integer", "minimum": 0}}, additionalProperties=True), record({"foo": {"type": "string", "minLength": 1}}, additionalProperties=True)]},
        [{"foo": "ok", "bar": None}, {"foo": "ok", "bar": "text"}, {"foo": "ok", "bar": -1}, {"bar": 1, "foo": None}],
        [{"foo": "", "bar": -1}, {"foo": 1, "bar": None}, {}])
    add("union_guard_serialization", {"anyOf": [record({"x": {"type": "integer", "minimum": 0}}), record({"x": {"type": "integer", "maximum": 2}})]},
        [{"x": -1}, {"x": 1}, {"x": 3}], [{"x": None}, {"x": True}, {}], inline=False)
    add("overlapping_union", {"oneOf": [{"type": "integer", "minimum": 0}, {"type": "number", "maximum": 2}]},
        [-1, 1.5, 3], [0, 1, 2, True, "x"])
    add("anyof_annotations", {"type": "object", "anyOf": [{"properties": {"a": {"type": "integer"}}, "required": ["a"]},
        {"properties": {"b": {"type": "string"}}, "required": ["b"]}], "unevaluatedProperties": False},
        [{"a": 1}, {"b": "x"}, {"a": 1, "b": "x"}], [{}, {"a": 1, "b": 2}, {"a": 1, "c": 0}], inline=False)
    add("conditional_annotations", {"type": "object", "if": {"properties": {"tag": {"const": "a"}}, "required": ["tag"]},
        "then": {"properties": {"value": {"type": "integer"}}, "required": ["value"]},
        "else": {"properties": {"other": {"type": "string"}}}, "unevaluatedProperties": False},
        [{"tag": "a", "value": 1}, {}, {"other": "x"}], [{"tag": "a"}, {"tag": "b"}, {"value": 1}], inline=False)
    add("if_without_required", {"type": "object", "if": {"properties": {"flag": {"const": True}}}, "then": {"required": ["x"]}, "else": {"required": ["y"]}},
        [{"x": 1}, {"flag": True, "x": 1}, {"flag": False, "y": 1}], [{}, {"flag": True}, {"flag": False, "x": 1}])
    add("dependencies", {"type": "object", "dependentRequired": {"a": ["b"]}, "dependentSchemas": {"b": {"properties": {"a": {"type": "integer"}}}}},
        [{}, {"b": 1}, {"a": 2, "b": 1}], [{"a": 1}, {"a": "x", "b": 1}], inline=False)
    add("overlapping_patterns", {"type": "object", "properties": {"xy": {"maximum": 4}},
        "patternProperties": {"^x": {"type": "integer", "minimum": 1}, "y$": {"multipleOf": 2}}, "additionalProperties": False},
        [{}, {"xy": 2}, {"x": 3}, {"y": "not a number"}], [{"xy": 3}, {"xy": 6}, {"x": 0}, {"z": 2}], inline=False)
    add("property_names", {"type": "object", "propertyNames": {"pattern": "^[a-z]{1,3}$"}, "additionalProperties": {"type": "integer"}},
        [{}, {"abc": 1}], [{"abcd": 1}, {"A": 1}, {"a": True}])
    add("tuple_contains_annotations", {"type": "array", "prefixItems": [{"type": "integer"}], "contains": {"type": "string"},
        "minContains": 1, "maxContains": 2, "unevaluatedItems": False}, [[1, "x"], [1, "x", "y"]],
        [[], [1], [True, "x"], [1, "x", 2], [1, "x", "y", "z"]])
    add("contains_zero", {"type": "array", "contains": {"type": "integer"}, "minContains": 0, "maxContains": 0},
        [[], [True, "x", None]], [[0], [1.0], [True, 1]])
    add("unique_json_equality", {"type": "array", "uniqueItems": True},
        [[], [True, 1], [False, 0], [{"x": 1}, {"x": 2}]], [[1, 1.0], [0.0, -0.0], [{"a": 1, "b": 2}, {"b": 2, "a": 1}], [[1], [1.0]]])
    add("ref_siblings", {"$defs": {"value": {"type": "integer", "minimum": 0}}, "$ref": "#/$defs/value", "maximum": 2}, [0, 2], [-1, 3, True])
    add("embedded_resource", {"$id": "https://example.com/root", "$defs": {"a": {"$id": "child", "$anchor": "value", "type": "string", "minLength": 2}}, "$ref": "child#value"}, ["ok", "猫🐲"], ["", "x", 1])
    add("dynamic_scope", {"$id": "https://example.com/strict", "$dynamicAnchor": "node", "$ref": "tree", "unevaluatedProperties": False, "$defs": {"tree": {"$id": "tree", "$dynamicAnchor": "node", "type": "object", "properties": {"children": {"type": "array", "items": {"$dynamicRef": "#node"}}}}}}, [{}, {"children": [{}]}, {"children": [{"children": [{}]}]}], [{"children": [{"extra": 0}]}, {"extra": 0}, {"children": [1]}], inline=False)
    add("format_annotations", record({"email": {"type": "string", "format": "email", "minLength": 1}}),
        [{"email": "not-an-email"}, {"email": "ok@example.com"}], [{"email": ""}, {"email": 1}])
    add("dynamic_siblings", {"$id": "https://example.com/derived", "$ref": "base", "$defs": {
        "derived": {"$dynamicAnchor": "extra", "prefixItems": [True, {"type": "string"}]},
        "base": {"$id": "base", "type": "array", "prefixItems": [{"type": "string"}], "$dynamicRef": "#extra", "unevaluatedItems": False,
                 "$defs": {"default": {"$dynamicAnchor": "extra"}}}}}, [["a", "b"], ["a"], []], [[None, "b"], ["a", 1], ["a", "b", "c"]], inline=False)
    add("content_annotation", {"type": "string", "contentSchema": {"$ref": "https://example.com/unused"}}, ["", "not JSON"], [1, {}])
    recursive = record({"v": {"type": "integer", "minimum": 0}, "children": {"type": "array", "items": {"$ref": "#"}}})
    add("recursive", recursive, [{"v": 0, "children": []}, {"v": 1, "children": [{"v": 2, "children": []}]}],
        [{"v": 0, "children": [{"v": -1, "children": []}]}, {"v": 0, "children": [{}]}])
    add("mutual_refs", {"$ref": "#/$defs/a", "$defs": {
        "a": record({"a": {"type": "integer", "minimum": 0}, "next": {"$ref": "#/$defs/b"}}, required=["a"]),
        "b": record({"b": {"type": "string", "minLength": 1}, "next": {"$ref": "#/$defs/a"}}, required=["b"])}},
        [{"a": 0}, {"a": 1, "next": {"b": "x", "next": {"a": 2}}}], [{"a": 1, "next": {"b": ""}}, {"a": -1}])
    for i, pattern in enumerate([r"^(?=.{2,4}$)(?!ab)[a-z]+$", r"(?<=a)b(?=c)", r"(?<!a)b(?!c)", r"^(?:(?=a)a|b){1,3}$", r"^(?=a*)a*$", r"\bcat\b", r"\Bcat\B", r"(?m)^cat$", r"^(?=🐲)🐲{1,2}$"]):
        # These cases use the independent validator to label an exhaustive
        # bounded alphabet; hand-labeled cases above also audit that oracle.
        add(f"regex_{i}", {"type": "string", "pattern": pattern}, [], [], inline=False)
    add("integer_bound_wires", {"type": "number", "minimum": 1, "maximum": 2}, [1, 1.5, 2], [0, 3], wires=[
        ["0.999999999999999999999999999999999", False], ["2.000000000000000000000000000000001", False]])
    add("exact_decimal", {"type": "number", "exclusiveMaximum": 0.1}, [-1, 0, 0.09], [0.1, 1], wires=[
        ["0.099999999999999999999999999999999999", True], ["0.100000000000000000000000000000000001", False],
        ["1e-1", False], ["-0.0", True]])
    add("integer_wires", {"type": "integer", "minimum": 0, "maximum": 2**64}, [0, 2**64], [-1, True], wires=[
        ["1.0", True], ["1e2", True], ["1.5", False], ["1.0000000000000000000000000000000001", False], ["18446744073709551616", True], ["18446744073709551617", False]])
    fields = {f"field{i:03}": {"type": "integer", "minimum": i, "maximum": i + 2} for i in range(200)}
    value = {k: i for i, k in enumerate(fields)}
    add("wide_distinct_bounds", record(fields), [value], [dict(value, field199=198), dict(value, field000=3)], stress=True, inline=True)
    item = record({"payload": {"oneOf": [left, right]}, "text": {"type": "string", "minLength": 1}})
    rows = [{"payload": {"kind": "left", "v": i}, "text": "猫🐲" * 512} for i in range(256)]
    invalid = json.loads(json.dumps(rows)); invalid[-1]["payload"]["v"] = -1
    add("large_union_values", {"type": "array", "items": item}, [rows], [invalid], stress=True)
    return result


if __name__ == "__main__":
    from generated_schema_cases import cases as generated_cases
    print(json.dumps(cases() + list(generated_cases()), ensure_ascii=True, allow_nan=False))
