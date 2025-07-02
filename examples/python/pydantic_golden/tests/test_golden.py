import importlib
import pkgutil
import demo
from types import ModuleType
import json
import os


def _import_all_modules_from_package(package: ModuleType) -> None:
    """Recursively import all modules in a package."""
    for _, module_name, is_pkg in pkgutil.walk_packages(
        package.__path__, package.__name__ + "."
    ):
        importlib.import_module(module_name)


_import_all_modules_from_package(demo)

GOLDEN_PATH = os.path.join(os.path.dirname(__file__), "schemas.golden")


def test_golden_is_up_to_date(registry):
    assert len(registry) > 0

    if not os.path.exists(GOLDEN_PATH):
        raise FileNotFoundError(f"{GOLDEN_PATH} file not found.")

    with open(GOLDEN_PATH, "r", encoding="utf-8") as f:
        old_golden_data = json.load(f)

    new_golden_data = {
        k: {"stable_id": v.stable_id, "mode": v.mode, "schema": v.schema}
        for k, v in sorted(registry.items())
    }

    # If the golden data is not up to date, update it automatically
    if old_golden_data != new_golden_data:
        with open(GOLDEN_PATH, "w", encoding="utf-8") as f:
            json.dump(new_golden_data, f, indent=2, sort_keys=True)

    assert (
        old_golden_data == new_golden_data
    ), "Schemas.golden is not up to date. Automatically updated; re-running this test should pass."
