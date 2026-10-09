"""Differential E2E checks: original schemas, independent oracle, two runtime paths.

Plan rewriting is deliberately confined to tests. Removing both proof flags and
scalar guards prevents the optimized and general runs from sharing a bad guard.
Every serialization check starts independently of checked deserialization.
"""

import copy
import importlib.util
import itertools
import json
import random
import sys
from collections import Counter
from decimal import Decimal
from pathlib import Path
from unittest.mock import patch

import jsoncompat
from jsoncompat.codegen import dataclasses as dc

REPO = Path(__file__).resolve().parents[2]
COUNTS = Counter()
SEED = 0xC0DEC0DE
PRIMITIVES = [None, False, True, -2, -1, 0, 1, 2, 3, 1.5, -0.0, "", "a", "cat", "猫🐲", [], {}]


def wire(value):
    return json.dumps(value, ensure_ascii=True, allow_nan=False, separators=(",", ":"))


def exact_wire(value):
    """Preserve numeric lexemes from schemas/official JSON test inputs."""
    if isinstance(value, Decimal):
        return str(value)
    if isinstance(value, dict):
        return '{' + ','.join(wire(k) + ':' + exact_wire(v) for k, v in value.items()) + '}'
    if isinstance(value, list):
        return '[' + ','.join(map(exact_wire, value)) + ']'
    return wire(value)


def same_json(left, right):
    # JSON Schema equality distinguishes booleans from numbers, but not 1/1.0.
    if isinstance(left, bool) or isinstance(right, bool):
        return type(left) is type(right) and left == right
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return left == right
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(same_json(v, right[k]) for k, v in left.items())
    if isinstance(left, list):
        return len(left) == len(right) and all(same_json(a, b) for a, b in zip(left, right))
    return left == right


def load(path, mode, *, poison_schema=False):
    name = f"_differential_{COUNTS['imports']}_{mode}"
    COUNTS['imports'] += 1
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    original = dc.bind_prepared_model_runtimes
    proofs = {}

    def bind(roots, descriptors, frozen_list, frozen_dict, prepared):
        plan = json.loads(prepared)
        proofs.update((model, plan['conversion_validates'][node]) for model, node in roots)
        if mode == 'general':
            plan['conversion_validates'] = [False] * plan['base_nodes']
            plan['guard_nodes'] = []
            plan['guards'] = []
        if poison_schema:
            descriptors = list(descriptors)
            for _, node in roots:
                descriptor = list(descriptors[node])
                schema = json.loads(descriptor[-1])
                schema['nodes'][0]['rules'] = ['false']
                descriptor[-1] = wire(schema).encode()
                descriptors[node] = tuple(descriptor)
        return original(roots, descriptors, frozen_list, frozen_dict, wire(plan).encode())

    sys.modules[name] = module
    try:
        with patch.object(dc, 'bind_prepared_model_runtimes', bind):
            spec.loader.exec_module(module)
        return module, proofs[module.JSONCOMPAT_MODEL]
    except BaseException:
        sys.modules.pop(name, None)
        raise


def accepted(call):
    try:
        return True, call()
    except (ValueError, TypeError, OverflowError) as error:
        return False, str(error)


def require_acceptance(call, expected, context):
    ok, result = accepted(call)
    COUNTS[context[2]] += 1
    assert ok == expected, (context, 'expected', expected, 'actual', ok, result)
    return result if ok else None


def mutations(value):
    """Single-point changes, including absent fields and deep invalid leaves."""
    yield from PRIMITIVES
    if isinstance(value, dict):
        yield dict(reversed(list(value.items())))
        yield dict(value, **{'unknown/~field': 0})
        for key, child in value.items():
            yield {k: v for k, v in value.items() if k != key}
            for changed in mutations(child):
                yield dict(value, **{key: changed})
    elif isinstance(value, list):
        yield list(reversed(value))
        yield value + value[:1]
        for i, child in enumerate(value):
            yield value[:i] + value[i + 1:]
            for changed in mutations(child):
                yield value[:i] + [changed] + value[i + 1:]
    elif type(value) in (int, float):
        yield value - 1
        yield value + 1


def random_value(rng, depth=0):
    if depth < 3:
        kind = rng.randrange(5)
        if kind == 0:
            return [random_value(rng, depth + 1) for _ in range(rng.randrange(4))]
        if kind == 1:
            return {rng.choice(['x', 'a', 'b', 'value', 'kind', 'next']): random_value(rng, depth + 1)
                    for _ in range(rng.randrange(4))}
    return copy.deepcopy(rng.choice(PRIMITIVES))


def candidates(case):
    values = case['valid'] + case['invalid'] + case.get('samples', [])
    if not case.get('stress'):
        values = values + PRIMITIVES
        for value in case['valid'][:6]:
            values.extend(itertools.islice(mutations(value), 140))
        rng = random.Random(SEED)
        values.extend(random_value(rng) for _ in range(60))
    if case['name'].startswith('regex_'):
        alphabet = 'abcé🐲 \n'
        values.extend(''.join(chars) for n in range(5) for chars in itertools.product(alphabet, repeat=n))
        values.extend(['cat', 'écaté', ' cat!', '\x00cat\x00', 'cat\ncat', '🐲cat🐲'])
    seen = set()
    for value in values:
        key = wire(value)
        if key not in seen:
            seen.add(key)
            yield value


def check_case(case, path):
    schema = case['schema']
    oracle = jsoncompat.validator_for(case.get('schema_wire') or wire(schema))
    for label in ('valid', 'invalid'):
        for value in case[label]:
            assert oracle.is_valid_json(wire(value)) == (label == 'valid'), (case['name'], 'incorrect witness', label, value, schema)
    values = [(v, oracle.is_valid_value(v), oracle.is_valid_json(wire(v))) for v in candidates(case)]
    if case.get('independent'):
        from generated_schema_cases import matches
        for value, valid_value, valid_wire in values:
            expected = matches(schema, value)
            assert valid_value == valid_wire == expected, (case['name'], 'independent oracle', schema, value)
        COUNTS['independent_schemas'] += 1
    for mode in ('optimized', 'general'):
        module, inline = load(path, mode)
        try:
            model = module.JSONCOMPAT_MODEL
            if case.get('inline') is not None:
                assert inline == case['inline'], (case['name'], 'unexpected proof', inline)
            COUNTS['inline_models' if inline else 'fallback_models'] += 1
            for value, valid_value, valid_wire in values:
                context = (case['name'], mode, '', wire(value)[:1200])
                for op, call, expected in (
                    ('from_value', lambda: model.from_value(value), valid_value),
                    ('deserialize_str', lambda: model.deserialize(wire(value)), valid_wire),
                    ('deserialize_bytes', lambda: model.deserialize(wire(value).encode()), valid_wire),
                ):
                    instance = require_acceptance(call, expected, (*context[:2], op, context[3]))
                    if expected:
                        output = instance.to_value()
                        assert same_json(output, value), (context, op, 'changed value', output)
                        assert oracle.is_valid_value(output), (context, op, 'invalid output', output)
                # Trusted construction still enforces representability (types,
                # missing required slots, union selection). Count such refusals
                # explicitly; every schema-valid value must be representable.
                representable, instance = accepted(lambda: model.from_value(value, skip_validation=True))
                if not representable:
                    COUNTS['structurally_unrepresentable'] += 1
                    assert not valid_value, (context, 'valid trusted construction failed', instance)
                    continue
                for op, call in (('serialize', instance.serialize), ('to_value', instance.to_value)):
                    result = require_acceptance(call, valid_value, (*context[:2], op, context[3]))
                    if valid_value:
                        output = json.loads(result) if op == 'serialize' else result
                        assert same_json(output, value), (context, op, 'changed value', output)
                        assert oracle.is_valid_value(output), (context, op, 'invalid emitted value', output)
            for raw, expected in case.get('wires', []):
                assert oracle.is_valid_json(raw) == expected, (case['name'], 'incorrect wire witness', raw)
                for payload in (raw, raw.encode()):
                    instance = require_acceptance(lambda: model.deserialize(payload), expected,
                                                  (case['name'], mode, 'lexical_json', raw))
                    if expected:
                        # Python floats can round across an exact JSON bound.
                        # Checked serialization must reject that rounded value
                        # rather than emitting JSON that violates the schema.
                        rounded = instance.to_value(skip_validation=True)
                        emitted = require_acceptance(instance.serialize, oracle.is_valid_value(rounded),
                                                     (case['name'], mode, 'lexical_serialize', raw))
                        if emitted is not None:
                            assert oracle.is_valid_json(emitted), (case['name'], raw, emitted)
        finally:
            sys.modules.pop(module.__name__, None)
    COUNTS['schemas'] += 1
    COUNTS['candidates'] += len(values)


def check_path_controls(directory):
    for mode, expected in [('optimized', True), ('general', False)]:
        module, inline = load(directory / 'control.py', mode, poison_schema=True)
        try:
            assert inline, 'control must exercise the inline proof'
            model = module.JSONCOMPAT_MODEL
            require_acceptance(lambda: model.deserialize('{"value":1}'), expected, ('control', mode, 'path_control'))
            instance = model.from_value({'value': 1}, skip_validation=True)
            require_acceptance(instance.serialize, expected, ('control', mode, 'path_control'))
        finally:
            sys.modules.pop(module.__name__, None)


def check_mutated_models(directory):
    for mode in ('optimized', 'general'):
        module, _ = load(directory / 'recursive.py', mode)
        try:
            model = module.JSONCOMPAT_MODEL
            for replacement in (-1, True, 'bad', None, float('nan'), float('inf')):
                instance = model.from_value({'v': 0, 'children': [{'v': 1, 'children': []}]})
                object.__setattr__(instance.children[0], 'v', replacement)
                for op in ('serialize', 'to_value'):
                    require_acceptance(getattr(instance, op), False, ('mutated_recursive', mode, op, repr(replacement)))
            for raw in ('', '{', 'null trailing', '{"v":NaN}', '[1,]', '"\\ud800"'):
                require_acceptance(lambda: model.deserialize(raw), False, ('malformed', mode, 'deserialize_str', raw))
        finally:
            sys.modules.pop(module.__name__, None)


def check_fixture_corpus(fixtures):
    samples = json.loads((REPO / 'pybindings/bench_fixture_samples.json').read_text())
    count = 0
    for path in sorted(fixtures.rglob('*.py')):
        relative = path.relative_to(fixtures)
        kind = relative.parts[0]
        valid, invalid, seeds, wires = [], [], [], []
        original = None
        if kind == 'fuzz':
            text = (REPO / 'tests/fixtures' / relative.parent).with_suffix('.json').read_text()
            document = json.loads(text, parse_float=Decimal)
            fixture = document[int(relative.stem)] if isinstance(document, list) else document
            original = exact_wire(fixture['schema'])
            schema = json.loads(original)
            # Preserve official labels on original JSON lexemes. Converting a
            # decimal to a Python float may change its membership, so test the
            # rounded Python value separately against the same exact schema.
            validator = jsoncompat.validator_for(original)
            for test in fixture.get('tests', []):
                raw = exact_wire(test['data'])
                if relative.parts[1:3] == ('optional', 'format') or relative.parts[1:3] == ('optional', 'format-assertion'):
                    # These upstream files opt into asserted formats. The
                    # generated default dialect treats format as annotation;
                    # tests/fuzz.rs audits their assertion-mode labels.
                    wires.append((raw, validator.is_valid_json(raw)))
                    COUNTS['annotation_mode_witnesses'] += 1
                else:
                    wires.append((raw, test['valid']))
                value = json.loads(raw)
                (valid if validator.is_valid_value(value) else invalid).append(value)
        elif kind == 'backcompat':
            original = (REPO / 'tests/fixtures' / relative).with_suffix('.json').read_text()
            schema = json.loads(original)
        elif kind == 'benchmarks':
            original = (REPO / 'pybindings/benchmark_schemas' / relative.name).with_suffix('.json').read_text()
            schema = json.loads(original)
        else:
            # Stamped reader/writer models have direction-specific API tests.
            assert relative.parts[:2] == ('examples', 'stamp'), relative
            continue
        sample = samples.get(relative.with_suffix('').as_posix())
        if sample is not None:
            seeds.append(sample['value'])
        check_case(dict(name=relative.as_posix(), schema=schema, valid=valid,
                        invalid=invalid, samples=seeds, wires=wires, schema_wire=original), path)
        count += 1
    assert count > 500, count
    COUNTS['fixture_schemas'] = count


def main():
    directory = Path(sys.argv[1])
    check_path_controls(directory)
    for case in json.loads((directory / 'cases.json').read_text()):
        try:
            check_case(case, directory / (case['name'] + '.py'))
        except AssertionError:
            if case.get('independent'):
                from generated_schema_cases import minimize_failure
                minimize_failure(case, directory)
            raise
    check_mutated_models(directory)
    check_fixture_corpus(directory / "fixtures")
    assert COUNTS['schemas'] > 130, COUNTS
    assert COUNTS['inline_models'] > 20 and COUNTS['fallback_models'] > 20, COUNTS
    print('Differential E2E coverage (seed=%#x): %s' % (SEED, dict(sorted(COUNTS.items()))), flush=True)


if __name__ == '__main__':
    main()
