"""Prepared-model regressions, invoked by the Rust integration test."""

import ast
import hashlib
import importlib.util
import py_compile
import builtins
import dataclasses
import gc
import inspect
import json
import pickle
import sys
import unittest
import weakref
import zlib
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from unittest.mock import patch

import jsoncompat
from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen import dataclasses as dc

WORK = Path(sys.argv.pop(1))
REPO = Path(__file__).resolve().parents[2]
FIXTURES = REPO / "tests/fixtures/dataclasses"


def _load(path):
    name = "_generated_test_" + hashlib.sha256(str(path).encode()).hexdigest()
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        sys.modules.pop(name, None)
        raise
    return module


def outcome(model, value):
    """Include checked input AND checked output in the differential result."""
    try:
        instance = model.deserialize(json.dumps(value, ensure_ascii=False))
        return ("accepted", json.loads(instance.serialize()), instance.to_value())
    except (TypeError, ValueError):
        return ("rejected",)


class PreparedTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.modules = {}
        sys.path.insert(0, str(WORK))
        for source in WORK.glob("*.py"):
            py_compile.compile(str(source), doraise=True)
            if not source.name.startswith("_"):
                cls.modules[source.stem] = _load(source)

    def test_no_compiler_or_reflection_at_import_or_first_use(self):
        destination = WORK / "no_compile.py"
        destination.write_bytes((WORK / "constrained.py").read_bytes())
        py_compile.compile(str(destination), doraise=True)

        def forbidden(*args, **kwargs):
            raise AssertionError("prepared startup invoked a compiler/reflection")

        with patch.object(builtins, "compile", forbidden), patch.object(
            dataclasses, "dataclass", forbidden
        ), patch.object(inspect, "get_annotations", forbidden), patch.object(
            jsoncompat, "validator_for", forbidden
        ), patch.object(zlib, "decompress", forbidden):
            module = _load(destination)
            self.assertEqual(
                module.Constrained.deserialize('{"name":"x","count":1}').serialize(),
                '{"count":1,"name":"x"}',
            )
            with self.assertRaises(ValueError):
                module.Constrained.deserialize('{"name":"","count":-1}')
            with ThreadPoolExecutor(max_workers=4) as pool:
                self.assertEqual(
                    list(
                        pool.map(
                            lambda _: module.Constrained(name="x", count=1).serialize(),
                            range(32),
                        )
                    ),
                    ['{"count":1,"name":"x"}'] * 32,
                )

    def test_dataclass_protocol_and_formats(self):
        model = self.modules["constrained"].Constrained
        self.assertTrue(dataclasses.is_dataclass(model))
        self.assertTrue(model.__dataclass_params__.frozen)
        self.assertIs(model.__dataclass_params__, dc.DataclassModel.__dataclass_params__)
        for name in ("__repr__", "__eq__", "__hash__", "__setattr__", "__delattr__",
                     "__getstate__", "__setstate__", "__match_args__", "__dataclass_params__"):
            self.assertNotIn(name, model.__dict__)
        self.assertEqual(set(model.__dataclass_fields__), set(model.__slots__))
        self.assertNotIn("__jsoncompat_prepared_schema__", model.__dict__)
        signature = inspect.signature(model)
        self.assertEqual(signature.parameters["name"].kind, inspect.Parameter.KEYWORD_ONLY)
        self.assertEqual(signature.parameters["optional"].default, dc.JSONCOMPAT_MISSING)
        self.assertTrue(all(field.kw_only for field in dataclasses.fields(model)))
        self.assertEqual([field.name for field in dataclasses.fields(model)], ["count", "name", "optional"])
        item = model(name="hello", count=2)
        self.assertIn("name='hello'", repr(item))
        self.assertEqual(item, pickle.loads(pickle.dumps(item)))
        self.assertEqual(hash(item), hash(model(name="hello", count=2)))
        self.assertEqual(dataclasses.replace(item, count=3).count, 3)
        self.assertEqual(
            dataclasses.asdict(item),
            {"name": "hello", "count": 2, "optional": dc.JSONCOMPAT_MISSING},
        )
        with self.assertRaises(dataclasses.FrozenInstanceError):
            item.name = "changed"
        for format in SerializationFormat:
            self.assertEqual(
                model.deserialize(item.serialize(format=format), format=format), item
            )
        self.assertEqual(
            model(name="hello", count=2, optional=None).to_value()["optional"], None
        )
        self.assertNotIn("optional", item.to_value())

    def test_empty_dataclass_and_replace_preserve_constructor_options(self):
        empty = self.modules["empty"].Empty
        self.assertEqual(dataclasses.fields(empty), ())
        self.assertEqual(dataclasses.asdict(empty()), {})
        self.assertEqual(dataclasses.replace(empty()), empty())
        self.assertEqual(repr(empty()), "Empty()")
        self.assertEqual(hash(empty()), hash(()))
        self.assertEqual(empty().serialize(), "{}")
        model = self.modules["constrained"].Constrained
        item = model(name="valid", count=1)
        with self.assertRaises(ValueError):
            dataclasses.replace(item, count=-1)
        trusted = dataclasses.replace(item, count=-1, skip_validation=True)
        self.assertEqual(trusted.count, -1)
        with self.assertRaises(ValueError):
            trusted.serialize()

    def test_original_schema_is_only_expanded_on_explicit_access(self):
        destination = WORK / "schema_introspection.py"
        destination.write_bytes((WORK / "constrained.py").read_bytes())
        module = _load(destination)
        model = module.Constrained
        source = model.__dict__["__jsoncompat_schema__"]
        self.assertNotIsInstance(source, str)
        item = model(name="hello", count=1)
        with patch.object(zlib, "decompress", wraps=zlib.decompress) as decompress:
            schema = item.__jsoncompat_schema__
            self.assertIs(schema, model.__jsoncompat_schema__)
            self.assertIs(schema, item.__jsoncompat_schema__)
            self.assertEqual(decompress.call_count, 1)
        self.assertEqual(json.loads(schema)["title"], "Constrained")
        self.assertTrue(schema.startswith("{\n"))
        self.assertLess(len(source._compressed), len(schema.encode()))
        self.assertEqual(jsoncompat.validator_for(schema).is_valid_value(item.to_value()), True)
        # Source inspection must not affect equality, pickle, or field metadata.
        self.assertEqual(item, pickle.loads(pickle.dumps(item)))
        self.assertNotIn("__jsoncompat_schema__", model.__dataclass_fields__)

    def test_current_state_is_validated_even_after_trusted_construction(self):
        model = self.modules["constrained"].Constrained
        for payload in [{"name": "", "count": 1}, {"name": "x", "count": -1}]:
            with self.assertRaises(ValueError):
                model.deserialize(json.dumps(payload))
            with self.assertRaises(ValueError):
                model.from_value(payload)
            unchecked = model.from_value(payload, skip_validation=True)
            with self.assertRaises(ValueError):
                unchecked.serialize()
            with self.assertRaises(ValueError):
                unchecked.to_value()
            self.assertEqual(
                json.loads(unchecked.serialize(skip_validation=True)), payload
            )
        instance = model(name="x", count=0)
        object.__setattr__(instance, "name", "")
        with self.assertRaises(ValueError):
            instance.serialize()
        with self.assertRaises(ValueError):
            instance.to_value()
        object.__setattr__(instance, "name", [1])
        with self.assertRaises((ValueError, TypeError)):
            instance.serialize()

    def test_streaming_json_syntax_and_duplicate_keys(self):
        model = self.modules["constrained"].Constrained
        valid = '{"name":"🐲\\n\\"\\\\","count":123456789012345678901234567890}'
        for payload in (valid, valid.encode(), " \n" + valid + "\t"):
            self.assertEqual(model.deserialize(payload).to_value(), json.loads(valid))
        for payload in (
            '{"name":"x","count":1,"name":"y"}',
            '{"name":"x","count":1,"count":2}',
            '{"name":"x","count":1,}',
            '{"name":"x","count":01}',
            '{"name":"x","count":1e999}',
            '{"name":"x","count":NaN}',
            '{"name":"\\uD800","count":1}',
            '{"name":"x","count":1} true',
            '{"name":"x","count":1',
            '{"name":"x" "count":1}',
            b'{"name":"\xff","count":1}',
        ):
            for trusted in (False, True):
                with self.subTest(payload=payload, trusted=trusted):
                    with self.assertRaises((ValueError, TypeError)):
                        model.deserialize(payload, skip_validation=trusted)
        extras = self.modules["extras"].Extras
        for trusted in (False, True):
            with self.assertRaises(ValueError):
                extras.deserialize(
                    '{"name":"x","extra":"a","extra":"b"}',
                    skip_validation=trusted,
                )

    def test_string_emission_and_arbitrary_property_order(self):
        model = self.modules["constrained"].Constrained
        for offset in range(16):
            for codepoint in list(range(128)) + [0xE9, 0x2801, 0x1F432, 0x10FFFF]:
                text = "a" * offset + chr(codepoint) + "é🐲z"
                value = {"optional": None, "name": text, "count": 1}
                instance = model.deserialize(json.dumps(value, ensure_ascii=False))
                self.assertEqual(json.loads(instance.serialize()), value)
        # Exercise the large object's name lookup when the next field differs
        # from the serializer's order, including the last field's constraint.
        model = self.modules["wide"].JSONCOMPAT_MODEL
        value = {
            f"field{index:04}": "x" if index % 2 == 0 else index
            for index in reversed(range(1000))
        }
        self.assertEqual(
            json.loads(model.deserialize(json.dumps(value)).serialize()), value
        )
        value["field0000"] = ""
        with self.assertRaises(ValueError):
            model.deserialize(json.dumps(value))

    def test_extra_properties_and_subclasses_use_emitted_values(self):
        model = self.modules["extras"].Extras
        item = model(name="valid")
        object.__setattr__(item, "__jsoncompat_extra__", dc.FrozenDict({"name": ""}))
        with self.assertRaises(ValueError):
            item.serialize()
        self.assertEqual(item.serialize(skip_validation=True), '{"name":""}')
        object.__setattr__(item, "name", "")
        object.__setattr__(
            item, "__jsoncompat_extra__", dc.FrozenDict({"name": "valid"})
        )
        self.assertEqual(item.serialize(), '{"name":"valid"}')

        class Text(str):
            def __str__(self):
                return ""

        item = model(name=Text("valid"))
        self.assertEqual(item.serialize(), '{"name":"valid"}')

    def test_regex_and_ambiguous_unions(self):
        model = self.modules["regex"].JSONCOMPAT_MODEL
        for text in ("x", "xx", "xxx"):
            self.assertEqual(model.deserialize(json.dumps(text)).to_value(), text)
        for text in ("", "xxxx", "y", "x\n", "🐲"):
            with self.assertRaises(ValueError):
                model.deserialize(json.dumps(text))
        for name, values in [
            ("ambiguous", [{"value": -1}, {"value": 0}, {"value": 2}]),
            ("nested_regex", [{"x1": "abc"}, {}, {"xx": "z"}]),
            ("escaped", [{'a"b\\c\n🐲': 1}]),
        ]:
            for value in values:
                self.assertEqual(
                    outcome(self.modules[name].JSONCOMPAT_MODEL, value),
                    ("accepted", value, value),
                )
        with self.assertRaises(ValueError):
            self.modules["nested_regex"].JSONCOMPAT_MODEL.deserialize('{"xx":"A"}')

    def test_loaded_plans_are_collectable(self):
        destination = WORK / "collectable.py"
        destination.write_bytes((WORK / "constrained.py").read_bytes())
        module = _load(destination)
        ref = weakref.ref(module.Constrained)
        module.Constrained(name="x", count=1)
        sys.modules.pop(module.__name__)
        del module
        gc.collect()
        self.assertIsNone(ref())

    def test_large_schemas_and_multi_megabyte_values(self):
        def record(width, row=0):
            return {
                f"field{index:04}": (
                    f"row-{row}-field-{index}-" + "abc🐲" * 12
                    if index % 2 == 0
                    else row + index
                )
                for index in range(width)
            }

        for name, value in [
            ("wide", record(1000)),
            ("many", {f"section{index:04}": record(12, index) for index in range(200)}),
            ("records", [record(12, index) for index in range(12000)]),
        ]:
            with self.subTest(workload=name):
                wire = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
                if name == "records":
                    self.assertGreater(len(wire.encode()), 5_000_000)
                model = self.modules[name].JSONCOMPAT_MODEL
                instance = model.deserialize(wire)
                self.assertEqual(json.loads(instance.serialize()), value)
                self.assertEqual(instance.to_value(), value)
                last = instance
                if name == "many":
                    last = instance.section0199
                elif name == "records":
                    last = instance.root[-1]
                object.__setattr__(last, "field0000", "")
                with self.assertRaises(ValueError):
                    instance.serialize()
                with self.assertRaises(ValueError):
                    instance.to_value()

    def test_corrupt_programs_are_rejected_when_loaded(self):
        destination = WORK / "corrupt.py"
        destination.write_bytes((WORK / "constrained.py").read_bytes())
        py_compile.compile(str(destination), doraise=True)
        tree = ast.parse(destination.read_text())
        changed = False
        for call in ast.walk(tree):
            if isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute) and call.func.attr == "bind_module":
                for node in call.args[2].elts:
                    if ast.literal_eval(node.elts[0]) in ("model", "root"):
                        program = json.loads(ast.literal_eval(node.elts[-1]))
                        program["nodes"][0]["rules"].append({"ref": 999999})
                        node.elts[-1] = ast.Constant(json.dumps(program).encode())
                        changed = True
        self.assertTrue(changed)
        destination.write_text(ast.unparse(tree))
        with self.assertRaisesRegex(ValueError, "invalid node reference"):
            _load(destination)

    def test_split_module_is_readable_and_equivalent(self):
        source = (WORK / "constrained_public.py").read_text()
        self.assertNotIn("dc.install_model", source)
        self.assertNotIn("__jsoncompat_schema__", source)
        self.assertNotIn("def _jsoncompat_init", source)
        self.assertLess(len(source), 2000)
        model = self.modules["constrained_public"].Constrained
        item = model(name="x", count=2)
        self.assertEqual(item.serialize(), '{"count":2,"name":"x"}')
        self.assertEqual(item, pickle.loads(pickle.dumps(item)))

    def test_independent_models_do_not_copy_unrelated_definitions(self):
        module = self.modules["independent"]
        self.assertNotIn("$defs", json.loads(module.Other.__jsoncompat_schema__))
        self.assertEqual(module.Other(name="x").serialize(), '{"name":"x"}')
        self.assertEqual(module.Independent(value=1).serialize(), '{"value":1}')

    def test_fields_cannot_shadow_generated_helpers(self):
        model = self.modules["reserved"].JSONCOMPAT_MODEL
        value = {"dc":"module", "self":1, "dict":"mapping", "str":"text", "typing":"annotations"}
        instance = model.deserialize(json.dumps(value))
        self.assertEqual(json.loads(instance.serialize()), value)
        self.assertEqual(instance.dc_, "module")
        self.assertEqual(instance.self_, 1)
        self.assertEqual(instance.dict_, "mapping")
        self.assertEqual(instance.str_, "text")

    def test_recursive_models(self):
        model = self.modules["recursive"].JSONCOMPAT_MODEL
        value = {"value": 1, "children": [{"value": 2, "children": []}]}
        instance = model.deserialize(json.dumps(value))
        self.assertIs(type(instance.children[0]), model)
        self.assertEqual(json.loads(instance.serialize()), value)
        with self.assertRaises(ValueError):
            model.deserialize('{"value":1,"children":[{"value":-1,"children":[]}]}')

    def test_decimal_json_boundaries_are_checked_before_float_conversion(self):
        module = _load(FIXTURES / "fuzz/optional/bignum/004.py")
        model = module.JSONCOMPAT_MODEL
        invalid = "972783798187987123879878123.188781371"
        valid = "972783798187987123879878122.18878137"
        for wire in (invalid, invalid.encode()):
            with self.assertRaises(ValueError):
                model.deserialize(wire)
        model.deserialize(valid).serialize()
        # A Python float has already lost the original lexeme. Its actual
        # value is below the boundary and the Python-value API accepts it.
        rounded = json.loads(invalid)
        self.assertTrue(jsoncompat.validator_for(model.__jsoncompat_schema__).is_valid_value(rounded))
        self.assertEqual(model.from_value(rounded).root, rounded)

    def test_entire_generated_fixture_corpus(self):
        samples = json.loads((REPO / "pybindings/bench_fixture_samples.json").read_text())
        checked_modules = checked_examples = 0
        for source in sorted(FIXTURES.rglob("*.py")):
            relative = source.relative_to(FIXTURES)
            with self.subTest(fixture=str(relative)):
                module = _load(source)
                try:
                    model = module.JSONCOMPAT_MODEL
                    # Writer/reader direction restrictions have dedicated tests.
                    if issubclass(model, (dc.WriterDataclassModel, dc.ReaderDataclassModel, dc.ReaderDataclassRootModel)):
                        continue
                    validator = jsoncompat.validator_for(model.__jsoncompat_schema__)
                    cases = [None, False, True, 0, 1, 1.5, "", "abc", [], {}]
                    if relative.parts[0] == "fuzz":
                        fixture = json.loads((REPO / "tests/fixtures" / relative.parent).with_suffix(".json").read_text())
                        case = fixture[int(relative.stem)] if isinstance(fixture, list) else fixture
                        cases.extend(test["data"] for test in case.get("tests", []))
                    sample = samples.get(str(relative.with_suffix("")))
                    if sample is not None:
                        value = sample["value"]
                        cases.append(value)
                        if isinstance(value, dict):
                            cases.extend({k: v for k,v in value.items() if k != missing} for missing in value)
                            cases.extend(dict(value, **{key: bad}) for key in value for bad in [None,False,0,"",[],{}])
                    for value in cases:
                        actual = outcome(model, value)
                        expected = validator.is_valid_value(value)
                        self.assertEqual(actual[0] == "accepted", expected, (relative, value, actual))
                        if expected:
                            self.assertEqual(actual[1:], (value, value), (relative, value))
                        checked_examples += 1
                    checked_modules += 1
                finally:
                    sys.modules.pop(module.__name__, None)
        self.assertGreater(checked_modules, 500)
        self.assertGreater(checked_examples, 8000)
        print(f"Generated corpus: {checked_modules} modules, {checked_examples} validator comparisons")


if __name__ == "__main__":
    unittest.main()
