"""Check each crate's numeric feature contract from outside the workspace.

Run after Cargo has fetched the locked workspace dependencies. Only Cargo metadata is
resolved; numeric behavior is covered by the runtime suites. These consumers
have no workspace dev-dependencies that could accidentally enable precision.
"""

from __future__ import annotations

import json
from pathlib import Path
import shutil
import subprocess
import tempfile


REPOSITORY = Path(__file__).resolve().parents[1]
PACKAGES = {
    "jsoncompat": ".",
    "json_schema_ast": "schema",
    "json_schema_fuzz": "fuzz",
    "jsoncompat_codegen": "codegen",
    "jsoncompat_openapi": "openapi",
    "jsoncompat_py": "pybindings",
    "jsoncompat_wasm": "wasm",
}


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="jsoncompat-features-") as directory:
        consumer = Path(directory)
        (consumer / "src").mkdir()
        (consumer / "src" / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
        for package, path in PACKAGES.items():
            for mode in ("default", "precision"):
                options = ', features = ["arbitrary-precision"]' if mode == "precision" else ""
                dependency_path = json.dumps(str(REPOSITORY / path))
                (consumer / "Cargo.toml").write_text(
                    '[package]\nname = "precision-consumer"\nversion = "0.0.0"\nedition = "2024"\n'
                    '[workspace]\n[dependencies]\n'
                    f'{package} = {{ path = {dependency_path}{options} }}\n',
                    encoding="utf-8",
                )
                shutil.copyfile(REPOSITORY / "Cargo.lock", consumer / "Cargo.lock")
                metadata = json.loads(subprocess.check_output(
                    ["cargo", "metadata", "--offline", "--format-version=1"],
                    cwd=consumer, text=True,
                ))
                packages = {item["id"]: item for item in metadata["packages"]}
                nodes = metadata["resolve"]["nodes"]
                for dependency, feature in (("serde_json", "arbitrary_precision"),
                                            ("jsonschema", "arbitrary-precision")):
                    dependency_nodes = [node for node in nodes
                                        if packages[node["id"]]["name"] == dependency]
                    assert dependency_nodes, (package, mode, "missing", dependency)
                    for node in dependency_nodes:
                        assert (feature in node["features"]) == (mode == "precision"), (
                            package, mode, node["id"], node["features"],
                        )
                print(f"{package}: {mode} passed", flush=True)


if __name__ == "__main__":
    main()
