"""Compare generated packages without importing models or retaining expanded schemas.

Schema introspection text must be byte-for-byte identical. Prepared validation
programs must be graph-bisimilar from their model roots: node and pattern IDs
may change, but every rule, constant, pattern, diagnostic reference, and edge
must be preserved. This checks storage and graph restructuring independently of
sample request data; it does not prove correctness of the original programs.

Run after a GitHub scale experiment:
    python pybindings/bench_codegen_equivalence.py --before OLD --after NEW --output REPORT.json
Run the verifier's own regression checks with --self-test.
"""

from __future__ import annotations
import argparse
import ast
import collections
import copy
import hashlib
import json
import time
import tempfile
import unittest
import zlib
from decimal import Decimal
from pathlib import Path


def ensure(condition: bool, message: object) -> None:
    if not condition:
        raise AssertionError(message)


class Package:
    def __init__(self, root: Path):
        self.root = root
        self.chunks: dict[Path, bytes] = {}
        self.streams: dict[bytes, bytes] = {}
        self.manifest = json.loads((root / ".jsoncompat-manifest.json").read_text())

    def model_files(self, directory: Path) -> dict[str, Path]:
        files = list((self.root / directory).glob("_init_*.py"))
        if files:
            return {path.name.removeprefix("_init_"): path for path in files}
        return {
            path.name: path for path in (self.root / directory).glob("*.py")
            if "\ndef _jsoncompat_install(" in path.read_text()
        }

    def data(self, path: Path) -> bytes:
        if path not in self.chunks:
            tree = ast.parse(path.read_text())
            assignments = [
                n for n in tree.body if isinstance(n, (ast.Assign, ast.AnnAssign))
            ]
            ensure(len(assignments) == 1, path)
            value = ast.literal_eval(assignments[0].value)
            ensure(isinstance(value, bytes), path)
            self.chunks[path] = value
        return self.chunks[path]

    def initializer(self, path: Path, *, omit_context=False) -> tuple[bytes, bytes]:
        namespace: dict[str, bytes] = {}
        schema = program = None

        def value(node):
            if isinstance(node, ast.Name):
                return namespace[node.id]
            if isinstance(node, (ast.Tuple, ast.List)):
                return tuple(value(e) for e in node.elts)
            if isinstance(node, ast.Constant):
                return node.value
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
                if node.func.attr == "SchemaSource":
                    parts = value(node.args[0])
                    expanded = []
                    for part in parts:
                        compressed = b"".join(part)
                        key = hashlib.sha256(compressed).digest()
                        if key not in self.streams:
                            self.streams[key] = zlib.decompress(compressed)
                        expanded.append(schema_fragment(self.streams[key], omit_context))
                    return b"".join(expanded)
                if node.func.attr == "join":
                    ensure(
                        value(node.func.value) == b"", "value(node.func.value) == b''"
                    )
                    return b"".join(value(node.args[0]))
            raise AssertionError((path, ast.dump(node)))

        for node in ast.parse(path.read_text()).body:
            if isinstance(node, ast.ImportFrom):
                if node.module and node.module.startswith("_schemas."):
                    source = self.root / Path(*node.module.split(".")).with_suffix(
                        ".py"
                    )
                elif node.module and node.module.startswith("_program_"):
                    source = path.parent / (node.module + ".py")
                else:
                    continue
                ensure(len(node.names) == 1 and node.names[0].name == "DATA", path)
                namespace[node.names[0].asname] = self.data(source)
            elif isinstance(node, (ast.Assign, ast.AnnAssign)):
                target = (
                    node.targets[0] if isinstance(node, ast.Assign) else node.target
                )
                if not isinstance(target, ast.Name):
                    continue
                if target.id in {"_schema", "_jsoncompat_schema"}:
                    source = value(node.value)
                    if isinstance(node.value, ast.Call):
                        schema = source
                    else:
                        schema = zlib.decompress(
                            b"".join(source) if isinstance(source, tuple) else source
                        )
                elif target.id in {"_program", "_jsoncompat_program"}:
                    program = value(node.value)
                elif target.id.startswith(("_program_", "_schema_", "_jsoncompat_program_", "_jsoncompat_schema_")):
                    namespace[target.id] = value(node.value)
        ensure(isinstance(schema, bytes) and isinstance(program, bytes), path)
        return schema, program


class ResourcePackage(Package):
    """Read co-located declarations and packed constants without executing code."""

    def __init__(self, root: Path):
        super().__init__(root)
        self.constants = {}
        self.models = {}
        self.aliases = {}
        for filename in self.manifest["files"]:
            path = root / filename
            if not filename.startswith(("models/", "operations/", "_cycles/")) or path.suffix != ".py":
                continue
            namespace = {}
            for node in ast.parse(path.read_text()).body:
                if isinstance(node, ast.ImportFrom) and node.module:
                    if node.module.startswith("_data.s_"):
                        data_path = root / (node.module.replace(".", "/") + ".py")
                        if data_path not in self.constants:
                            self.constants[data_path] = {
                                item.target.id: ast.literal_eval(item.value)
                                for item in ast.parse(data_path.read_text()).body
                                if isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name)
                            }
                        for alias in node.names:
                            namespace[alias.asname or alias.name] = self.constants[data_path][alias.name]
                    elif node.module == "models._empty_headers":
                        for alias in node.names:
                            if alias.asname and alias.asname != alias.name:
                                self.aliases[alias.asname] = alias.name
                if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
                    name = node.targets[0].id
                    if name.startswith("_jsoncompat_schema_"):
                        def chunks(expr):
                            ensure(isinstance(expr, ast.Tuple), (path, ast.dump(expr)))
                            return tuple(namespace[item.id] for item in expr.elts)
                        if isinstance(node.value, ast.Call):
                            ensure(isinstance(node.value.func, ast.Attribute) and node.value.func.attr == "SchemaSource", path)
                            namespace[name] = tuple(chunks(part) for part in node.value.args[0].elts)
                        else:
                            namespace[name] = (chunks(node.value),)
                    elif name.startswith("_jsoncompat_program_"):
                        model = name.removeprefix("_jsoncompat_program_")
                        program = ast.literal_eval(node.value)
                        ensure(isinstance(program, bytes), path)
                        self.models[model] = (namespace[f"_jsoncompat_schema_{model}"], program)

    def model(self, name, *, omit_context=False):
        parts, program = self.models[self.aliases.get(name, name)]
        expanded = []
        for part in parts:
            compressed = b"".join(part)
            key = hashlib.sha256(compressed).digest()
            if key not in self.streams:
                self.streams[key] = zlib.decompress(compressed)
            expanded.append(schema_fragment(self.streams[key], omit_context))
        return b"".join(expanded), program


def schema_fragment(data, omit_context):
    if omit_context:
        for keyword in (b'"$defs"', b'"definitions"'):
            if data.lstrip().startswith(keyword + b": "):
                return b"  " + keyword + b": {}"
    return data


def normalize_header_names(value, aliases, *, program=False):
    def pointer(text):
        for prefix in ("#/$defs/", "/$defs/"):
            if isinstance(text, str) and text.startswith(prefix):
                name, *suffix = text[len(prefix):].split("/", 1)
                if name in aliases:
                    return prefix + aliases[name] + ("/" + suffix[0] if suffix else "")
        return text
    result = copy.deepcopy(value)
    if program:
        for node in result["nodes"]:
            for rule in node["rules"]:
                if "ref" in rule:
                    rule["ref"]["pointer"] = pointer(rule["ref"]["pointer"])
        return result
    if not isinstance(result, dict):
        return result
    # Walk schema-bearing keywords only. Objects in const/enum/examples remain
    # literal data even when they contain keys named $ref or x-jsoncompat.
    for key in ("properties", "patternProperties", "$defs", "definitions", "dependentSchemas"):
        if isinstance(result.get(key), dict):
            result[key] = {name: normalize_header_names(schema, aliases) for name, schema in result[key].items()}
    for key in ("allOf", "anyOf", "oneOf", "prefixItems"):
        if isinstance(result.get(key), list):
            result[key] = [normalize_header_names(schema, aliases) for schema in result[key]]
    for key in ("not", "if", "then", "else", "items", "contains", "additionalProperties", "unevaluatedProperties", "unevaluatedItems", "propertyNames", "contentSchema"):
        if key in result:
            result[key] = normalize_header_names(result[key], aliases)
    for key in ("$ref", "$dynamicRef"):
        if key in result:
            result[key] = pointer(result[key])
    metadata = result.get("x-jsoncompat")
    if isinstance(metadata, dict) and metadata.get("kind") == "declaration":
        if "name" in metadata:
            metadata["name"] = aliases.get(metadata["name"], metadata["name"])
        for key in ("stable_id", "schema_ref"):
            if key in metadata:
                metadata[key] = pointer(metadata[key])
    return result


def compare_resource_layout(old, new, output, started):
    previous = {}
    for group in old.manifest["modules"]:
        for path in old.model_files(Path(*group.split("."))).values():
            tree = ast.parse(path.read_text())
            classes = [n.name for n in tree.body if isinstance(n, ast.ClassDef)]
            ensure(len(classes) == 1, path)
            previous[classes[0]] = path
    ensure(set(previous) == (set(new.models) - set(new.aliases.values())) | set(new.aliases), "class inventory differs beyond declared empty-header aliases")
    counts = dict(models=0, new_classes=len(new.models), empty_header_aliases=len(new.aliases),
                  prepared_programs_byte_identical=0, schema_root_text_equal=0, prepared_node_pairs=0)
    for name, path in sorted(previous.items()):
        before_schema, before_program = old.initializer(path, omit_context=True)
        after_schema, after_program = new.model(name, omit_context=True)
        counts["schema_root_text_equal"] += before_schema == after_schema
        # Build namespaces can differ. Compare model roots separately; every
        # reachable dependency is checked by the complete prepared programs below.
        roots = []
        for schema in (before_schema, after_schema):
            root = json.loads(schema, parse_float=Decimal)
            if isinstance(root, dict):
                root = {key: value for key, value in root.items() if key not in {"$defs", "definitions"}}
            roots.append(normalize_header_names(root, new.aliases))
        ensure(exact_json(roots[0]) == exact_json(roots[1]), (name, "model schema root differs"))
        counts["prepared_programs_byte_identical"] += before_program == after_program
        a, b = (normalize_header_names(json.loads(data, parse_float=Decimal), new.aliases, program=True) for data in (before_program, after_program))
        counts["prepared_node_pairs"] += compare_programs(a, b, name)
        counts["models"] += 1
        if counts["models"] % 500 == 0:
            print(json.dumps({"progress": counts["models"], "seconds": time.perf_counter() - started}), flush=True)
    result = dict(**counts, model_schema_roots_equal=True, prepared_programs_bisimilar=True,
                  diagnostic_normalization="only renamed empty-header definition references",
                  before=str(old.root), after=str(new.root), seconds=time.perf_counter()-started,
                  before_compiler=old.manifest["compiler"], after_compiler=new.manifest["compiler"])
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result), flush=True)


def compare_resource_packages(old, new, output, started):
    """Stronger check for optimizations that retain the resource layout."""
    ensure(old.models, "no generated models found")
    ensure(old.models.keys() == new.models.keys(), "model inventory differs")
    ensure(old.aliases == new.aliases, "header aliases differ")
    ensure(old.manifest["exports"] == new.manifest["exports"], "public exports differ")
    counts = dict(models=0, new_classes=len(new.models), empty_header_aliases=len(new.aliases),
                  prepared_programs_byte_identical=0, prepared_node_pairs=0)
    checked_fragments = set()
    for name, (before_parts, before_program) in sorted(old.models.items()):
        after_parts, after_program = new.models[name]
        ensure(len(before_parts) == len(after_parts), (name, "schema fragment count differs"))
        for before, after in zip(before_parts, after_parts):
            pair = (b"".join(before), b"".join(after))
            if pair not in checked_fragments:
                ensure(zlib.decompress(pair[0]) == zlib.decompress(pair[1]), (name, "full schema text differs"))
                checked_fragments.add(pair)
        ensure(before_program == after_program, (name, "prepared program bytes differ"))
        counts["prepared_programs_byte_identical"] += 1
        counts["prepared_node_pairs"] += compare_programs(
            json.loads(before_program, parse_float=Decimal),
            json.loads(after_program, parse_float=Decimal), name,
        )
        counts["models"] += 1

    def module_code(package):
        result = {}
        for filename in package.manifest["files"]:
            if filename.startswith("_data/") or not filename.endswith((".py", ".pyi")):
                continue
            tree = ast.parse((package.root / filename).read_text())
            # Only compression and the names of its stored fragments may differ.
            # Constructors, metadata, complete conversion descriptors, binding
            # plans, and type declarations must be identical Python syntax.
            tree.body = [node for node in tree.body if not (
                isinstance(node, ast.ImportFrom) and node.module and node.module.startswith("_data.s_")
                or isinstance(node, ast.Assign) and len(node.targets) == 1
                and isinstance(node.targets[0], ast.Name)
                and node.targets[0].id.startswith("_jsoncompat_schema_")
            )]
            result[filename] = ast.dump(tree)
        return result
    before_code, after_code = module_code(old), module_code(new)
    ensure(before_code.keys() == after_code.keys(), "module inventory differs")
    for filename in before_code:
        ensure(before_code[filename] == after_code[filename], (filename, "generated module code differs"))
    result = dict(**counts, full_schema_text_equal=True, prepared_programs_bisimilar=True,
                  module_code_equal=True, modules_compared=len(before_code),
                  schema_fragments_compared=len(checked_fragments),
                  before=str(old.root), after=str(new.root), seconds=time.perf_counter()-started,
                  before_compiler=old.manifest["compiler"], after_compiler=new.manifest["compiler"])
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result), flush=True)


EDGE_SCALARS = {"not", "property_names", "unevaluated_properties", "unevaluated_items"}
EDGE_LISTS = {"all", "any", "one"}
LEAF_RULES = {
    "dependencies",
    "multiple_of",
    "string_length",
    "array_length",
    "object_length",
    "bound",
}


def exact_json(value) -> str:
    """Keep arbitrary-precision numeric literals distinct from strings and booleans."""
    if isinstance(value, Decimal):
        return str(value)
    if isinstance(value, dict):
        return (
            "{"
            + ",".join(
                json.dumps(key) + ":" + exact_json(value[key]) for key in sorted(value)
            )
            + "}"
        )
    if isinstance(value, list):
        return "[" + ",".join(exact_json(item) for item in value) + "]"
    return json.dumps(value, separators=(",", ":"))


def normalize_node(program, index):
    ensure(0 <= index < len(program["nodes"]), ("invalid node reference", index))
    node = copy.deepcopy(program["nodes"][index])
    edges = []

    def edge(index):
        if index is None:
            return None
        ensure(
            isinstance(index, int)
            and not isinstance(index, bool)
            and 0 <= index < len(program["nodes"]),
            ("invalid node reference", index),
        )
        edges.append(index)
        return "<node>"

    def pattern(index):
        ensure(
            isinstance(index, int)
            and not isinstance(index, bool)
            and 0 <= index < len(program["patterns"])
            and index < len(program["pattern_sources"]),
            ("invalid pattern reference", index),
        )
        return {
            "source": program["pattern_sources"][index],
            "compiled": program["patterns"][index],
        }

    for rule in node["rules"]:
        if isinstance(rule, str):
            ensure(rule in {"false", "unique"}, rule)
            continue
        ensure(len(rule) == 1, rule)
        kind, payload = next(iter(rule.items()))
        if kind in EDGE_SCALARS:
            rule[kind] = edge(payload)
        elif kind in EDGE_LISTS:
            rule[kind] = [edge(n) for n in payload]
        elif kind == "ref":
            payload["node"] = edge(payload["node"])
        elif kind == "if":
            for key in ("condition", "then_node", "else_node"):
                payload[key] = edge(payload[key])
        elif kind == "object":
            payload["properties"] = [
                [name, edge(n)] for name, n in payload["properties"]
            ]
            payload["patterns"] = [
                [pattern(p), edge(n)] for p, n in payload["patterns"]
            ]
            payload["additional"] = edge(payload["additional"])
        elif kind == "dependent_schemas":
            rule[kind] = [[name, edge(n)] for name, n in payload]
        elif kind == "array":
            payload["prefix"] = [edge(n) for n in payload["prefix"]]
            payload["items"] = edge(payload["items"])
        elif kind == "contains":
            payload["node"] = edge(payload["node"])
        elif kind == "pattern":
            rule[kind] = pattern(payload)
        else:
            ensure(kind in LEAF_RULES, kind)
    # Serialized comparisons distinguish booleans from numbers and preserve all
    # non-edge fields, rule order, branch order, patterns, and ref diagnostics.
    return exact_json(node), edges


def compare_programs(left, right, name):
    allowed = {"version", "nodes", "patterns", "pattern_sources", "exact_json_numbers"}
    for program in (left, right):
        ensure(set(program) <= allowed, (name, "unknown prepared program fields"))
    ensure(left["version"] == right["version"], name)
    ensure(
        left.get("exact_json_numbers", False) == right.get("exact_json_numbers", False),
        name,
    )
    pending = collections.deque([(0, 0)])
    visited = set()
    caches = ({}, {})
    while pending:
        a, b = pending.popleft()
        if (a, b) in visited:
            continue
        visited.add((a, b))
        for program, index, cache in ((left, a, caches[0]), (right, b, caches[1])):
            if index not in cache:
                cache[index] = normalize_node(program, index)
        signature_a, edges_a = caches[0][a]
        signature_b, edges_b = caches[1][b]
        ensure(signature_a == signature_b, (name, a, b, signature_a, signature_b))
        ensure(len(edges_a) == len(edges_b), (name, a, b))
        pending.extend(zip(edges_a, edges_b))
    return len(visited)


class VerifierTests(unittest.TestCase):
    def test_resource_comparison_checks_schema_program_and_conversion_code(self):
        with tempfile.TemporaryDirectory(prefix="jsoncompat-resource-equivalence-") as directory:
            base = Path(directory)
            schema = b'{"type":"string","minLength":2}'
            program = json.dumps({"version":4, "nodes":[{"types":["string"], "rules":[]}]}).encode()
            def package(name, level, *, source_schema=schema, source_program=program, annotation="str"):
                root = base / name
                (root / "models").mkdir(parents=True, exist_ok=True)
                (root / "_data").mkdir(exist_ok=True)
                files = ["models/example.py", "_data/s_a.py"]
                (root / ".jsoncompat-manifest.json").write_text(json.dumps({
                    "version":6, "compiler":name, "files":dict.fromkeys(files, "unused"),
                    "exports":{"Example":"models.example"},
                }))
                (root / "_data/s_a.py").write_text(f"S_{name}: bytes = {zlib.compress(source_schema, level)!r}\n")
                (root / "models/example.py").write_text(
                    f"from .._data.s_a import S_{name} as _jsoncompat_S_{name}\n"
                    f"class Example: value: {annotation}\n"
                    f"_jsoncompat_schema_Example = (_jsoncompat_S_{name},)\n"
                    f"_jsoncompat_program_Example = {source_program!r}\n"
                    "raise RuntimeError('never execute generated modules')\n"
                )
                return ResourcePackage(root)
            before = package("before", 9)
            output = base / "result.json"
            compare_resource_packages(before, package("after", 1), output, time.perf_counter())
            self.assertTrue(json.loads(output.read_text())["full_schema_text_equal"])
            for changes in [{"source_schema":b'{"type":"string"}'}, {"source_program":program+b" "}, {"annotation":"int"}]:
                with self.subTest(changes=changes), self.assertRaises(AssertionError):
                    compare_resource_packages(before, package("after", 1, **changes), output, time.perf_counter())

    def test_empty_header_normalization_does_not_rewrite_literal_values(self):
        aliases = {"GetPet404Headers": "JsoncompatEmptyHeaders"}
        literal = {"const": {"$ref": "#/$defs/GetPet404Headers"}, "enum": ["GetPet404Headers", "#/$defs/GetPet404Headers"]}
        self.assertEqual(normalize_header_names(literal, aliases), literal)
        self.assertEqual(normalize_header_names({"$ref": "#/$defs/GetPet404Headers"}, aliases), {"$ref": "#/$defs/JsoncompatEmptyHeaders"})

    def test_program_reindexing_and_meaningful_changes(self) -> None:
        integer = {"types": ["integer"], "choices": None, "rules": []}
        text = {"types": ["string"], "choices": None, "rules": [{"pattern": 0}]}
        left = {
            "version": 4,
            "nodes": [
                {
                    "types": None,
                    "choices": None,
                    "rules": [
                        {"any": [1, 2]},
                        {"ref": {"node": 0, "pointer": "/$defs/Root"}},
                    ],
                },
                integer,
                text,
            ],
            "patterns": [{"program": [1, 2, 3]}, {"program": [4]}],
            "pattern_sources": ["^a", "unused"],
        }
        right = {
            "version": 4,
            "nodes": [
                {
                    "types": None,
                    "choices": None,
                    "rules": [
                        {"any": [2, 1]},
                        {"ref": {"node": 0, "pointer": "/$defs/Root"}},
                    ],
                },
                {**text, "rules": [{"pattern": 1}]},
                integer,
            ],
            "patterns": list(reversed(left["patterns"])),
            "pattern_sources": list(reversed(left["pattern_sources"])),
        }
        self.assertEqual(compare_programs(left, right, "reindexed"), 3)
        for change in (
            "type",
            "branch",
            "pointer",
            "pattern",
            "pattern_source",
            "number_flag",
        ):
            with self.subTest(change=change):
                changed = copy.deepcopy(right)
                if change == "type":
                    changed["nodes"][2]["types"] = ["number"]
                elif change == "branch":
                    changed["nodes"][0]["rules"][0]["any"] = [1, 2]
                elif change == "pointer":
                    changed["nodes"][0]["rules"][1]["ref"]["pointer"] = "/$defs/Other"
                elif change == "pattern":
                    changed["patterns"][1]["program"][0] = 9
                elif change == "pattern_source":
                    changed["pattern_sources"][1] = "^b"
                else:
                    changed["exact_json_numbers"] = True
                with self.assertRaises(AssertionError):
                    compare_programs(left, changed, change)
        bool_constant = copy.deepcopy(left)
        numeric_constant = copy.deepcopy(left)
        bool_constant["nodes"][0]["choices"] = [True]
        numeric_constant["nodes"][0]["choices"] = [1]
        with self.assertRaises(AssertionError):
            compare_programs(bool_constant, numeric_constant, "boolean is not integer")
        precise = copy.deepcopy(left)
        rounded = copy.deepcopy(left)
        precise["nodes"][0]["choices"] = [Decimal("0.10000000000000001")]
        rounded["nodes"][0]["choices"] = [Decimal("0.1")]
        with self.assertRaises(AssertionError):
            compare_programs(precise, rounded, "numeric precision is preserved")
        unknown_rule = copy.deepcopy(left)
        unknown_rule["nodes"][0]["rules"].append({"future_edge": 2})
        with self.assertRaises(AssertionError):
            compare_programs(
                unknown_rule, unknown_rule, "new wire rules require support"
            )

    def test_static_reader_accepts_chunks_and_inline_data_without_execution(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(prefix="jsoncompat-equivalence-") as directory:
            root = Path(directory)
            (root / ".jsoncompat-manifest.json").write_text("{}")
            (root / "_schemas").mkdir()
            group = root / "_models" / "group"
            group.mkdir(parents=True)
            schema = b'{\n  "type": "object"\n}'
            program = b'{"version":4,"nodes":[]}'
            (root / "_schemas" / "_schema_old.py").write_text(
                f"DATA: bytes = {zlib.compress(schema)!r}\n"
            )
            (group / "_program_old_0000.py").write_text(f"DATA = {program!r}\n")
            before = group / "_init_before.py"
            before.write_text(
                "from ..._schemas._schema_old import DATA as _schema_0\n"
                "_schema: tuple[bytes, ...] = (_schema_0,)\n"
                "from ._program_old_0000 import DATA as _program_0\n"
                "_program = b''.join([_program_0])\n"
            )
            after = group / "after.py"
            after.write_text(
                "class Example: pass\n"
                "def _jsoncompat_install(model, namespace): raise RuntimeError('never execute artifacts')\n"
                f"_jsoncompat_schema_0 = {zlib.compress(schema[:5])!r}\n"
                f"_jsoncompat_schema_1 = {zlib.compress(schema[5:])!r}\n"
                "_jsoncompat_schema = dc.SchemaSource(((_jsoncompat_schema_0,), (_jsoncompat_schema_1,)))\n"
                f"_program_0 = {program[:7]!r}\n"
                f"_program = b''.join([_program_0, {program[7:]!r}])\n"
            )
            package = Package(root)
            self.assertEqual(package.initializer(before), (schema, program))
            self.assertEqual(package.initializer(after), (schema, program))
            after.write_text("_program = __import__('os').system('exit 0')\n")
            with self.assertRaises(AssertionError):
                package.initializer(after)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--before", type=Path, help="baseline generated package")
    parser.add_argument("--after", type=Path, help="new generated package")
    parser.add_argument("--output", type=Path, help="JSON verification report")
    parser.add_argument(
        "--self-test", action="store_true", help="run the verifier regression checks"
    )
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(VerifierTests)
        if not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful():
            raise SystemExit(1)
        return
    if not all((args.before, args.after, args.output)):
        parser.error(
            "--before, --after, and --output are required unless using --self-test"
        )
    started = time.perf_counter()
    old, new = Package(args.before), Package(args.after)
    if new.manifest["version"] >= 6:
        if old.manifest["version"] >= 6:
            compare_resource_packages(ResourcePackage(args.before), ResourcePackage(args.after), args.output, started)
            return
        compare_resource_layout(old, ResourcePackage(args.after), args.output, started)
        return
    ensure(
        old.manifest["modules"].keys() == new.manifest["modules"].keys(),
        "old.manifest['modules'].keys() == new.manifest['modules'].keys()",
    )
    ensure(
        old.manifest["exports"] == new.manifest["exports"],
        "old.manifest['exports'] == new.manifest['exports']",
    )
    counts = dict(
        models=0,
        prepared_node_pairs=0,
        prepared_programs_byte_identical=0,
        schema_bytes_compared=0,
        old_prepared_nodes=0,
        new_prepared_nodes=0,
    )
    for group in old.manifest["modules"]:
        directory = Path(*group.split("."))
        previous = old.model_files(directory)
        current = new.model_files(directory)
        ensure(previous, (group, "no model initializers found"))
        ensure(previous.keys() == current.keys(), group)
        for name, before in sorted(previous.items()):
            after = current[name]
            before_schema, before_program = old.initializer(before)
            after_schema, after_program = new.initializer(after)
            ensure(before_schema == after_schema, str(before))
            counts["prepared_programs_byte_identical"] += before_program == after_program
            a = json.loads(before_program, parse_float=Decimal)
            b = json.loads(after_program, parse_float=Decimal)
            counts["prepared_node_pairs"] += compare_programs(a, b, str(before))
            counts["models"] += 1
            counts["schema_bytes_compared"] += len(before_schema)
            counts["old_prepared_nodes"] += len(a["nodes"])
            counts["new_prepared_nodes"] += len(b["nodes"])
            if counts["models"] % 500 == 0:
                print(
                    json.dumps(
                        {
                            "progress": counts["models"],
                            "seconds": time.perf_counter() - started,
                        }
                    ),
                    flush=True,
                )
    result = dict(
        **counts,
        exact_schema_text_equal=True,
        prepared_programs_bisimilar=True,
        before=str(args.before),
        after=str(args.after),
        seconds=time.perf_counter() - started,
        before_compiler=old.manifest["compiler"],
        after_compiler=new.manifest["compiler"],
    )
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result), flush=True)


if __name__ == "__main__":
    main()
