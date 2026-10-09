"""Seeded schema/value generation with a small independent semantic oracle.

The grammar deliberately uses exact small numbers and portable literal patterns;
this oracle does not call either jsoncompat validator. Existing hand-written and
upstream fixtures cover the remainder of Draft 2020-12 and numeric lexemes.
"""
import copy
import json
import os
import random
import re

SEED = int(os.environ.get("JSONCOMPAT_SCHEMA_SEED", "3503345870"))
PRIMITIVES = [None, True, False, -3, -1, 0, 1, 2, 4, 0.5, "", "x", "é猫", [], {}]


def equal(a, b):
    if isinstance(a, bool) or isinstance(b, bool):
        return type(a) is type(b) and a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a == b
    if type(a) is not type(b):
        return False
    if isinstance(a, list):
        return len(a) == len(b) and all(equal(x, y) for x, y in zip(a, b))
    if isinstance(a, dict):
        return a.keys() == b.keys() and all(equal(v, b[k]) for k, v in a.items())
    return a == b


def matches(schema, value, root=None):
    if isinstance(schema, bool):
        return schema
    root = schema if root is None else root
    if "$ref" in schema:
        target = root
        for token in schema["$ref"][2:].split("/"):
            target = target[token.replace("~1", "/").replace("~0", "~")]
        if not matches(target, value, root):
            return False
    types = dict(null=value is None, boolean=type(value) is bool,
                 integer=type(value) in (int, float) and value % 1 == 0,
                 number=type(value) in (int, float), string=isinstance(value, str),
                 array=isinstance(value, list), object=isinstance(value, dict))
    if "type" in schema:
        kinds = schema["type"] if isinstance(schema["type"], list) else [schema["type"]]
        if not any(types[k] for k in kinds):
            return False
    if "const" in schema and not equal(value, schema["const"]):
        return False
    if "enum" in schema and not any(equal(value, x) for x in schema["enum"]):
        return False
    for key, check in (("allOf", all), ("anyOf", any), ("oneOf", lambda flags: sum(flags) == 1)):
        if key in schema and not check(matches(s, value, root) for s in schema[key]):
            return False
    if "not" in schema and matches(schema["not"], value, root):
        return False
    if "if" in schema:
        branch = "then" if matches(schema["if"], value, root) else "else"
        if not matches(schema.get(branch, True), value, root):
            return False
    if types["number"]:
        if value < schema.get("minimum", value) or value > schema.get("maximum", value):
            return False
        if "multipleOf" in schema and value % schema["multipleOf"] != 0:
            return False
    if types["string"]:
        if len(value) < schema.get("minLength", 0) or len(value) > schema.get("maxLength", len(value)):
            return False
        if "pattern" in schema and re.search(schema["pattern"], value) is None:
            return False
    if types["array"]:
        if len(value) < schema.get("minItems", 0) or len(value) > schema.get("maxItems", len(value)):
            return False
        prefix = schema.get("prefixItems", [])
        if not all(matches(prefix[i] if i < len(prefix) else schema.get("items", True), item, root) for i, item in enumerate(value)):
            return False
        if "contains" in schema:
            count = sum(matches(schema["contains"], item, root) for item in value)
            if count < schema.get("minContains", 1) or count > schema.get("maxContains", count):
                return False
        if schema.get("uniqueItems") and any(equal(value[i], value[j]) for i in range(len(value)) for j in range(i)):
            return False
    if types["object"]:
        if not set(schema.get("required", [])).issubset(value):
            return False
        props = schema.get("properties", {})
        if not all(matches(props.get(k, schema.get("additionalProperties", True)), v, root) for k, v in value.items()):
            return False
        for k, required in schema.get("dependentRequired", {}).items():
            if k in value and not set(required).issubset(value):
                return False
    return True


def schema(rng, depth):
    leaves = [True, False, {"type": "integer", "minimum": rng.randrange(-3, 3)},
              {"type": "number", "multipleOf": rng.choice([1, 2, 0.5])},
              {"type": "string", "minLength": rng.randrange(3), "pattern": rng.choice(["x", "^", "é"])},
              {"enum": rng.sample(PRIMITIVES[:13], 3)}, {"const": rng.choice(PRIMITIVES)}]
    if depth == 0 or rng.randrange(4) == 0:
        return copy.deepcopy(rng.choice(leaves))
    op = rng.randrange(8)
    if op < 3:
        return {("allOf", "anyOf", "oneOf")[op]: [schema(rng, depth - 1) for _ in range(2)]}
    if op == 3:
        return {"not": schema(rng, depth - 1)}
    if op == 4:
        return {"if": schema(rng, depth - 1), "then": schema(rng, depth - 1), "else": schema(rng, depth - 1)}
    if op == 5:
        return {"type": "array", "prefixItems": [schema(rng, depth - 1)], "items": schema(rng, depth - 1), "uniqueItems": rng.choice([True, False])}
    if op == 6:
        return {"type": "array", "contains": schema(rng, depth - 1), "minContains": rng.randrange(3), "maxContains": 3}
    return {"type": "object", "properties": {"x": schema(rng, depth - 1), "a/b~": schema(rng, depth - 1)},
            "required": rng.choice([[], ["x"], ["x", "a/b~"]]), "additionalProperties": rng.choice([True, False]),
            "dependentRequired": {"x": ["a/b~"]} if rng.randrange(2) else {}}


def value(rng, depth):
    if depth == 0 or rng.randrange(3) == 0:
        return copy.deepcopy(rng.choice(PRIMITIVES))
    if rng.randrange(2):
        return [value(rng, depth - 1) for _ in range(rng.randrange(4))]
    return {k: value(rng, depth - 1) for k in rng.sample(["x", "a/b~", "extra"], rng.randrange(4))}


def cases():
    rng = random.Random(SEED)
    for index in range(int(os.environ.get("JSONCOMPAT_SCHEMA_CASES", "128"))):
        base = schema(rng, 3)
        values = PRIMITIVES + [value(rng, 3) for _ in range(80)]
        valid = [v for v in values if matches(base, v)]
        invalid = [v for v in values if not matches(base, v)]
        variants = [base, {"not": {"not": base}}, {"allOf": [True, base]},
                    {"$defs": {"a/b~": base}, "$ref": "#/$defs/a~1b~0"}]
        for variant, s in enumerate(variants):
            yield dict(name=f"generated_{SEED}_{index}_{variant}", schema=s, valid=valid, invalid=invalid,
                       inline=None, wires=[], stress=True, independent=True, seed=SEED)


def reductions(value):
    """Finite structural shrinking, shared by schemas and failed instances."""
    if isinstance(value, dict):
        for key in value:
            yield {k: v for k, v in value.items() if k != key}
        for key, child in value.items():
            for reduced in reductions(child):
                yield dict(value, **{key: reduced})
    elif isinstance(value, list):
        for i in range(len(value)):
            yield value[:i] + value[i+1:]
        for i, child in enumerate(value):
            for reduced in reductions(child):
                yield value[:i] + [reduced] + value[i+1:]
    elif isinstance(value, str) and value:
        yield ""
        if len(value) > 1:
            yield value[:1]
    elif type(value) in (int, float) and value != 0:
        yield 0


if __name__ == "__main__":
    print(json.dumps(list(cases()), ensure_ascii=True))


def minimize_failure(case, directory):
    """Keep a small executable schema/value reproducer after an E2E mismatch.

    Only runs after failure. Every accepted reduction regenerates and executes
    both runtime paths, so the artifact preserves the actual discrepancy.
    """
    import subprocess
    from python_dataclasses_differential import load, accepted, wire
    import sys
    cli = os.environ["JSONCOMPAT_TEST_CLI"]
    source = directory / "minimized.py"
    schema_file = directory / "minimized.schema.json"
    probes = 0

    def disagreement(s, values):
        nonlocal probes
        probes += 1
        schema_file.write_text(json.dumps(s))
        build = subprocess.run([cli, "codegen", "--target", "dataclasses", str(schema_file)], capture_output=True, text=True)
        if build.returncode:
            return None
        source.write_text(build.stdout)
        for mode in ("optimized", "general"):
            module, _ = load(source, mode)
            try:
                model = module.JSONCOMPAT_MODEL
                for v in values:
                    expected = matches(s, v)
                    for op, run in (("from_value", lambda: model.from_value(v)),
                                    ("deserialize", lambda: model.deserialize(wire(v)))):
                        ok, result = accepted(run)
                        if ok != expected:
                            return v, mode, op
                        if ok and not equal(result.to_value(), v):
                            return v, mode, "round_trip_value"
                    ok, instance = accepted(lambda: model.from_value(v, skip_validation=True))
                    if ok:
                        for op, run in (("serialize", instance.serialize), ("to_value", instance.to_value)):
                            if accepted(run)[0] != expected:
                                return v, mode, op
            finally:
                sys.modules.pop(module.__name__, None)
        return None

    s = case["schema"]
    found = disagreement(s, case["valid"] + case["invalid"])
    if found is None:
        return
    v, mode, op = found
    while probes < 160:
        changed = False
        for smaller in reductions(v):
            found = disagreement(s, [smaller])
            if found:
                v, mode, op = found
                changed = True
                break
            if probes >= 160:
                break
        if changed:
            continue
        for smaller in reductions(s):
            try:
                found = disagreement(smaller, [v])
            except (KeyError, TypeError, ValueError):
                found = None  # Invalid reductions are not counterexamples.
            if found:
                s = smaller
                v, mode, op = found
                changed = True
                break
            if probes >= 160:
                break
        if not changed:
            break
    schema_file.write_text(json.dumps(s, indent=2))
    (directory / "minimized.failure.json").write_text(json.dumps(dict(seed=case["seed"], name=case["name"],
        schema=s, value=v, mode=mode, operation=op, expected=matches(s, v), probes=probes), indent=2))
