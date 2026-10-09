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

    def test_numeric_exponents_never_turn_failed_assertions_into_negations(self):
        for exponent in (324, 10000, 10001, 2147483648):
            for negative in (False, True):
                wire = f"{'-' if negative else ''}1e-{exponent}"
                for name, schema, expected in (
                    ("tiny_minimum", {"minimum": 0}, not negative),
                    ("tiny_not", {"not": {"minimum": 0}}, negative),
                    ("tiny_if", {"if": {"minimum": 0}, "then": False, "else": True}, negative),
                    ("tiny_integer", {"type": "integer"}, False),
                ):
                    with self.subTest(wire=wire, name=name):
                        self.assertEqual(jsoncompat.validator_for(json.dumps(schema)).is_valid_json(wire), expected)
                        model = self.modules[name].JSONCOMPAT_MODEL
                        if expected:
                            model.deserialize(wire)
                        else:
                            with self.assertRaises(ValueError):
                                model.deserialize(wire)

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
        ), patch.object(zlib, "decompress", forbidden), patch.object(
            dataclasses, "field", forbidden
        ):

            module = _load(destination)
            item = module.Constrained(name="x", count=1)
            self.assertTrue(dataclasses.is_dataclass(item))
            self.assertIn("name='x'", repr(item))
            self.assertEqual(item, module.Constrained(name="x", count=1))
            self.assertEqual(hash(item), hash(module.Constrained(name="x", count=1)))
            self.assertIs(module.Constrained.__init__.__kwdefaults__["optional"], dc.JSONCOMPAT_MISSING)
            self.assertIsNone(module.Constrained.__dataclass_fields__._materialized)
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

    def test_dataclass_reflection_is_cached_and_preserves_metadata(self):
        destination = WORK / "reflection.py"
        destination.write_bytes((WORK / "constrained.py").read_bytes())
        model = _load(destination).Constrained
        metadata = model.__dataclass_fields__
        self.assertIsNone(metadata._materialized)
        self.assertTrue(dataclasses.is_dataclass(model))
        self.assertIsNone(metadata._materialized)
        fields = dataclasses.fields(model)
        self.assertIsNotNone(metadata._materialized)
        self.assertTrue(all(a is b for a, b in zip(fields, dataclasses.fields(model))))
        self.assertEqual([field.type for field in fields], [model.__annotations__[field.name] for field in fields])
        optional = next(field for field in fields if field.name == "optional")
        self.assertIs(optional.default, dc.JSONCOMPAT_MISSING)
        self.assertEqual(dict(optional.metadata), {
            dc.JSONCOMPAT_FIELD_KIND_METADATA: "property",
            dc.JSONCOMPAT_JSON_NAME_METADATA: "optional",
            dc.JSONCOMPAT_MISSING_METADATA: True,
        })
        with self.assertRaises(TypeError):
            optional.metadata["changed"] = True
        extras = self.modules["extras"].Extras
        extra = next(field for field in dataclasses.fields(extras) if field.name == dc.JSONCOMPAT_EXTRA_FIELD)
        self.assertFalse(extra.repr)
        self.assertIs(extra.default_factory, dict)
        self.assertNotIn(dc.JSONCOMPAT_EXTRA_FIELD, repr(extras(name="x")))
        self.assertEqual(dataclasses.asdict(model(name="x", count=1))["name"], "x")

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

    def test_unicode_allocations_preserve_all_character_widths(self):
        model = self.modules["unicode_text"].JSONCOMPAT_MODEL
        values = [
            "a" * length + last
            for length in (0, 1, 31, 63, 64, 127, 1024)
            for last in ("a", "\x00", "é", "ÿ", "Ā", "\uffff", "🐲", "\U0010ffff")
        ]
        values += ["é" * 1000, "Ā" * 1000, "🐲" * 1000, "e\u0301" * 1000]
        # Sweep every Unicode scalar, including width boundaries, noncharacters,
        # and embedded ASCII controls. Surrogates are not valid UTF-8 scalars.
        values.append("".join(chr(c) for c in range(0x110000) if not 0xD800 <= c < 0xE000))
        for value in values:
            wire = json.dumps(value, ensure_ascii=False)
            for trusted in (False, True):
                instance = model.deserialize(wire, skip_validation=trusted)
                self.assertEqual(instance.root, value)
                self.assertEqual(hash(instance.root), hash(value))
                self.assertEqual(instance.root.encode(), value.encode())
                emitted = instance.serialize(skip_validation=trusted)
                self.assertIs(type(emitted), str)
                self.assertEqual(json.loads(emitted), value)
                self.assertEqual(json.loads(emitted.encode()), value)
                self.assertEqual(emitted, str(emitted))

    def test_shared_nodes_preserve_oneof_branch_multiplicity(self):
        impossible = self.modules["duplicate_oneof"].JSONCOMPAT_MODEL
        for value in (None, False, True, 0, 1, "foo", [], {}, {"x": "y"}):
            with self.assertRaises(ValueError):
                impossible.deserialize(json.dumps(value))
            with self.assertRaises(ValueError):
                impossible.from_value(value)
            trusted = impossible.from_value(value, skip_validation=True)
            with self.assertRaises(ValueError):
                trusted.serialize()
            self.assertEqual(json.loads(trusted.serialize(skip_validation=True)), value)
        model = self.modules["duplicate_leaf_oneof"].JSONCOMPAT_MODEL
        self.assertEqual(json.loads(model.deserialize("123").serialize()), 123)
        for value in ("x", "long enough", ""):
            with self.assertRaises(ValueError):
                model.deserialize(json.dumps(value))
            with self.assertRaises(ValueError):
                model.from_value(value, skip_validation=True).serialize()

    def test_repeated_constraints_share_checks_without_losing_field_validation(self):
        source = ast.parse((WORK / "wide.py").read_text())
        call = next(node for node in ast.walk(source) if isinstance(node, ast.Call)
                    and isinstance(node.func, ast.Attribute) and node.func.attr == "bind_module")
        plan = json.loads(ast.literal_eval(call.args[3]))
        descriptors = call.args[2].elts
        program = json.loads(ast.literal_eval(next(node for node in descriptors if ast.literal_eval(node.elts[0]) == "model").elts[-1]))
        self.assertLess(len(program["nodes"]), 10)
        self.assertEqual(len(plan["guards"]), 1000)
        self.assertEqual(len(plan["guard_nodes"]), 2)
        self.assertEqual(len(plan["conversion_validates"]), plan["base_nodes"] + 2)
        # Both shared constraints remain attached to every field, including
        # fields far from the first occurrence of each constraint.
        model = self.modules["wide"].JSONCOMPAT_MODEL
        valid = {f"field{i:04}": "x" if i % 2 == 0 else i for i in range(1000)}
        for index in (0, 1, 498, 499, 998, 999):
            bad = dict(valid, **{f"field{index:04}": "" if index % 2 == 0 else -1})
            with self.assertRaises(ValueError):
                model.deserialize(json.dumps(bad))
            with self.assertRaises(ValueError):
                model.from_value(bad, skip_validation=True).serialize()

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

    def test_word_boundaries_match_the_general_validator(self):
        values = ["", "cat", " cat!", "wildcat", "catfish", "écat", "caté", "_cat_", "猫 cat 猫", "cat\n", "🐲cat🐲"]
        for name in ("word_boundary", "non_boundary", "boundary_look", "boundary_start", "boundary_end", "boundary_start_half", "boundary_end_half"):
            model = self.modules[name].JSONCOMPAT_MODEL
            validator = jsoncompat.validator_for(model.__jsoncompat_schema__)
            for value in values:
                with self.subTest(pattern=name, value=value):
                    expected = validator.is_valid_value(value)
                    self.assertEqual(outcome(model, value)[0] == "accepted", expected)
                    trusted = model.from_value(value, skip_validation=True)
                    if expected:
                        self.assertEqual(json.loads(trusted.serialize()), value)
                    else:
                        with self.assertRaises(ValueError):
                            trusted.serialize()

    def test_regex_budget_exhaustion_cannot_accept_invalid_values(self):
        text = "a" * 600_000
        for name, value in (("budget_not", text), ("budget_if", text), ("budget_keys", {text: 1})):
            model = self.modules[name].JSONCOMPAT_MODEL
            validator = jsoncompat.validator_for(model.__jsoncompat_schema__)
            self.assertFalse(validator.is_valid_value(value), name)
            with self.subTest(schema=name):
                with self.assertRaises(ValueError):
                    model.deserialize(json.dumps(value))
                with self.assertRaises(ValueError):
                    model.from_value(value)
                trusted = model.from_value(value, skip_validation=True)
                with self.assertRaises(ValueError):
                    trusted.serialize()
                with self.assertRaises(ValueError):
                    trusted.to_value()
                self.assertEqual(json.loads(trusted.serialize(skip_validation=True)), value)

    def test_corrupt_shared_guards_are_rejected_at_import(self):
        for case in ("reference", "original", "owner", "field", "rule", "length"):
            with self.subTest(case=case):
                tree = ast.parse((WORK / "constrained.py").read_text())
                call = next(node for node in ast.walk(tree) if isinstance(node, ast.Call)
                            and isinstance(node.func, ast.Attribute) and node.func.attr == "bind_module")
                plan = json.loads(ast.literal_eval(call.args[3]))
                if case == "reference":
                    plan["guards"][0]["guard"] = len(plan["guard_nodes"])
                elif case == "original":
                    plan["guard_nodes"][0]["original"] = plan["base_nodes"]
                elif case == "owner":
                    plan["guards"][0]["owner"] = 999999
                elif case == "field":
                    plan["guards"][0]["field"] = 999999
                elif case == "rule":
                    plan["guard_nodes"][0]["guard"]["rules"] = [{"ref": 0}]
                else:
                    plan["conversion_validates"].pop()
                call.args[3] = ast.Constant(json.dumps(plan).encode())
                destination = WORK / f"corrupt_guard_{case}.py"
                destination.write_text(ast.unparse(tree))
                with self.assertRaises(ValueError):
                    _load(destination)

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
                    sample = samples.get(relative.with_suffix("").as_posix())
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
