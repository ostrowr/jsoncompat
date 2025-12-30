import json
import sys
import types
from pathlib import Path

import pytest


def repo_root() -> Path:
    here = Path(__file__).resolve()
    for parent in here.parents:
        if (parent / "tests" / "fixtures" / "fuzz").exists():
            return parent
    for parent in here.parents:
        if (parent / "Cargo.toml").exists():
            return parent
    raise RuntimeError("could not locate repo root (Cargo.toml or fixtures)")


ROOT = repo_root()
FIXTURES = ROOT / "tests" / "fixtures" / "fuzz"
GOLDENS = Path(__file__).resolve().parents[1]
BASE_MODULE_NAME = "json_schema_codegen_base"

WHITELIST: dict[str, set[int]] = {
    "vocabulary.json": {0},
    "properties.json": {2},
    "default.json": {0},
}


def load_base_module():
    """Ensure the shared base classes are importable by generated modules."""
    module = types.ModuleType(BASE_MODULE_NAME)
    code = (GOLDENS / f"{BASE_MODULE_NAME}.py").read_text(encoding="utf-8")
    exec(compile(code, f"{BASE_MODULE_NAME}.py", "exec"), module.__dict__)
    sys.modules[BASE_MODULE_NAME] = module
    return module


def collect_fixtures():
    for path in FIXTURES.rglob("*.json"):
        rel = path.relative_to(FIXTURES).as_posix()
        with path.open("r", encoding="utf-8") as fh:
            doc = json.load(fh)
        if isinstance(doc, list):
            for idx, entry in enumerate(doc):
                schema = entry.get("schema")
                tests = entry.get("tests", [])
                yield rel, idx, schema, tests
        else:
            yield rel, 0, doc, []


def load_serializer_module(rel_path: str, idx: int):
    serializer_path = GOLDENS / rel_path / f"{idx}_serializer.py"
    if not serializer_path.exists():
        pytest.skip(f"missing serializer golden for {rel_path}#{idx}")
    code = serializer_path.read_text(encoding="utf-8")
    glb: dict[str, object] = {}
    exec(compile(code, serializer_path.name, "exec"), glb)
    return glb


def find_serializer_class(glb: dict[str, object]):
    for obj in glb.values():
        if isinstance(obj, type) and obj.__name__.endswith("Serializer"):
            return obj
    return None


@pytest.fixture(scope="session", autouse=True)
def _base_module():
    return load_base_module()


@pytest.mark.parametrize(
    ("rel_path", "idx", "schema", "tests"),
    list(collect_fixtures()),
)
def test_serializers_accept_fixture_tests(rel_path: str, idx: int, schema, tests):
    if rel_path in WHITELIST and idx in WHITELIST[rel_path]:
        pytest.skip("whitelisted unsupported schema")
    if not tests:
        pytest.skip("no fixture tests available")

    glb = load_serializer_module(rel_path, idx)
    cls = find_serializer_class(glb)
    if cls is None:
        pytest.skip("no serializer class found")

    for test in tests:
        valid = bool(test.get("valid"))
        data = test.get("data")
        try:
            cls.model_validate_json(json.dumps(data))
            ok = True
        except Exception:
            ok = False
        assert ok == valid, f"{rel_path}#{idx} test {test.get('description', '')}"
