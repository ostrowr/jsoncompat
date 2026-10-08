"""Prepared-model regressions, invoked by the Rust integration test."""

import ast
import builtins
import dataclasses
import gc
import inspect
import json
import pickle
import sys
import unittest
import weakref
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from unittest.mock import patch

import jsoncompat
from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen import dataclasses as dc
from jsoncompat.codegen.build import _load, prepare_module

WORK = Path(sys.argv.pop(1))
REPO = Path(__file__).resolve().parents[2]
FIXTURES = REPO / "tests/fixtures/dataclasses"


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
        cls.sources = {}
        for source in WORK.glob("*.py"):
            destination = WORK / "prepared" / source.name
            prepare_module(source, destination)
            cls.modules[source.stem] = _load(destination)
            cls.sources[source.stem] = _load(source)

    def test_no_compiler_or_reflection_at_import_or_first_use(self):
        destination = WORK / "no_compile.py"
        prepare_module(WORK / "constrained.py", destination)

        def forbidden(*args, **kwargs):
            raise AssertionError("prepared startup invoked a compiler/reflection")

        with patch.object(builtins, "compile", forbidden), patch.object(
            dataclasses, "dataclass", forbidden
        ), patch.object(inspect, "get_annotations", forbidden), patch.object(
            dc, "_bind_generated_module", forbidden
        ), patch.object(
            dc, "compile_model_runtimes", forbidden
        ), patch.object(
            jsoncompat, "prepare_model_schema", forbidden
        ), patch.object(
            jsoncompat, "prepare_model_plan", forbidden
        ):
            module = _load(destination)
            self.assertEqual(
                module.Constrained.deserialize('{"name":"x","count":1}').serialize(),
                '{"count":1,"name":"x"}',
            )
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
        original = self.sources["constrained"].Constrained
        self.assertTrue(dataclasses.is_dataclass(model))
        self.assertEqual(inspect.signature(model), inspect.signature(original))
        self.assertEqual(model.__doc__, original.__doc__)
        self.assertEqual(
            str(model.__dataclass_params__), str(original.__dataclass_params__)
        )
        for actual, expected in zip(
            dataclasses.fields(model), dataclasses.fields(original), strict=True
        ):
            self.assertEqual(
                (actual.name, actual.type, actual.metadata, actual.kw_only),
                (expected.name, expected.type, expected.metadata, expected.kw_only),
            )
        item = model(name="hello", count=2)
        self.assertEqual(repr(item), repr(original(name="hello", count=2)))
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
                    outcome(self.sources[name].JSONCOMPAT_MODEL, value),
                )
        with self.assertRaises(ValueError):
            self.modules["nested_regex"].JSONCOMPAT_MODEL.deserialize('{"xx":"A"}')

    def test_build_is_atomic_and_loaded_plans_are_collectable(self):
        destination = WORK / "atomic.py"
        prepare_module(WORK / "constrained.py", destination)
        expected = destination.read_bytes()
        bad_source = WORK / "bad.py"
        bad_source.write_text("JSONCOMPAT_MODEL = None\n")
        with self.assertRaises(ValueError):
            prepare_module(bad_source, destination)
        self.assertEqual(destination.read_bytes(), expected)
        with self.assertRaises(ValueError):
            prepare_module(destination, destination)
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
        prepare_module(WORK / "constrained.py", destination)
        tree = ast.parse(destination.read_text())
        changed = False
        for statement in tree.body:
            if isinstance(statement, ast.Expr) and isinstance(
                statement.value, ast.Call
            ):
                call = statement.value
                if (
                    isinstance(call.func, ast.Name)
                    and call.func.id == "setattr"
                    and ast.literal_eval(call.args[1])
                    == "__jsoncompat_prepared_schema__"
                ):
                    program = json.loads(ast.literal_eval(call.args[2]))
                    program["nodes"][0]["rules"].append({"ref": 999999})
                    call.args[2] = ast.Constant(json.dumps(program).encode())
                    changed = True
        self.assertTrue(changed)
        destination.write_text(ast.unparse(tree))
        with self.assertRaisesRegex(ValueError, "invalid node reference"):
            _load(destination)

    def test_entire_generated_fixture_corpus(self):
        samples = json.loads(
            (REPO / "pybindings/bench_fixture_samples.json").read_text()
        )
        checked_modules = checked_examples = existing_limitations = 0
        for source in sorted(FIXTURES.rglob("*.py")):
            relative = source.relative_to(FIXTURES)
            destination = WORK / "corpus" / relative
            with self.subTest(fixture=str(relative)):
                original = _load(source)
                try:
                    try:
                        prepare_module(source, destination)
                    except ValueError:
                        # Some codegen snapshots retain unresolved URI refs;
                        # the original runtime rejects these schemas too.
                        with self.assertRaises(ValueError):
                            jsoncompat.validator_for(
                                original.JSONCOMPAT_MODEL.__jsoncompat_schema__
                            ).is_valid_value(None)
                        existing_limitations += 1
                        continue
                    prepared = _load(destination)
                    try:
                        cases = [None, False, True, 0, 1, 1.5, "", "abc", [], {}]
                        if relative.parts[0] == "fuzz":
                            fixture = json.loads(
                                (REPO / "tests/fixtures" / relative.parent)
                                .with_suffix(".json")
                                .read_text()
                            )
                            case = (
                                fixture[int(relative.stem)]
                                if isinstance(fixture, list)
                                else fixture
                            )
                            cases.extend(test["data"] for test in case.get("tests", []))
                        sample = samples.get(str(relative.with_suffix("")))
                        if sample is not None:
                            value = sample["value"]
                            cases.append(value)
                            if isinstance(value, dict):
                                cases.extend(
                                    {k: v for k, v in value.items() if k != missing}
                                    for missing in value
                                )
                                cases.extend(
                                    dict(value, **{key: bad})
                                    for key in value
                                    for bad in [None, False, 0, "", [], {}]
                                )
                        for value in cases:
                            self.assertEqual(
                                outcome(prepared.JSONCOMPAT_MODEL, value),
                                outcome(original.JSONCOMPAT_MODEL, value),
                                (relative, value),
                            )
                            checked_examples += 1
                        checked_modules += 1
                    finally:
                        sys.modules.pop(prepared.__name__, None)
                finally:
                    sys.modules.pop(original.__name__, None)
        self.assertEqual(
            checked_modules + existing_limitations, len(list(FIXTURES.rglob("*.py")))
        )
        print(
            f"Prepared corpus: {checked_modules} modules, {checked_examples} differential cases, {existing_limitations} existing reference limitations"
        )


if __name__ == "__main__":
    unittest.main()
