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
WHITELIST_PATH = Path(__file__).with_name("whitelist.json")


def load_whitelist() -> dict[str, dict[int, str]]:
    data = json.loads(WHITELIST_PATH.read_text(encoding="utf-8"))
    return {
        Path(rel).with_suffix("").as_posix(): {
            int(idx): reason for idx, reason in entries.items()
        }
        for rel, entries in data.items()
    }


WHITELIST = load_whitelist()


def load_base_module():
    """Ensure the shared base classes are importable by generated modules."""
    module = types.ModuleType(BASE_MODULE_NAME)
    code = (GOLDENS / f"{BASE_MODULE_NAME}.py").read_text(encoding="utf-8")
    exec(compile(code, f"{BASE_MODULE_NAME}.py", "exec"), module.__dict__)
    sys.modules[BASE_MODULE_NAME] = module
    return module


def collect_fixtures():
    for path in FIXTURES.rglob("*.json"):
        rel = path.relative_to(FIXTURES).with_suffix("").as_posix()
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
    module_key = (
        rel_path.replace("/", "_")
        .replace("-", "_")
        .replace(".", "_")
        .replace(" ", "_")
    )
    module_name = f"json_schema_codegen_{module_key}_{idx}_serializer"
    module = types.ModuleType(module_name)
    module.__file__ = str(serializer_path)
    sys.modules[module_name] = module
    exec(compile(code, serializer_path.name, "exec"), module.__dict__)
    return module.__dict__


def find_serializer_class(glb: dict[str, object]):
    for obj in glb.values():
        if isinstance(obj, type) and obj.__name__.endswith("Serializer"):
            return obj
    return None


@pytest.fixture(scope="session", autouse=True)
def _base_module():
    return load_base_module()


def whitelist_reason(rel_path: str, idx: int) -> str | None:
    entries = WHITELIST.get(rel_path)
    if not entries:
        return None
    return entries.get(idx)


@pytest.mark.parametrize(
    ("rel_path", "idx", "schema", "tests"),
    list(collect_fixtures()),
)
def test_serializers_accept_fixture_tests(rel_path: str, idx: int, schema, tests):
    if not tests:
        pytest.skip("no fixture tests available")

    reason = whitelist_reason(rel_path, idx)
    is_whitelisted = reason is not None

    glb = load_serializer_module(rel_path, idx)
    cls = find_serializer_class(glb)
    if cls is None:
        pytest.skip("no serializer class found")
    cls.model_rebuild()

    all_passed = True
    for test in tests:
        valid = bool(test.get("valid"))
        data = test.get("data")
        try:
            cls.model_validate_json(json.dumps(data))
            ok = True
        except Exception:
            ok = False
        success = ok == valid
        if not is_whitelisted:
            assert success, f"{rel_path}#{idx} test {test.get('description', '')}"
        else:
            all_passed &= success

    if is_whitelisted and all_passed:
        pytest.fail(
            f"Whitelisted failure now passes; please remove entry for {rel_path}#{idx}"
        )
