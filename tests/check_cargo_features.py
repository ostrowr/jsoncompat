"""Check each crate's numeric feature contract from outside the workspace.

Run after Cargo has fetched the locked workspace dependencies. These consumers
have no workspace dev-dependencies that could accidentally enable precision.
"""

from __future__ import annotations

import json
import os
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
PROBE = r'''
fn main() {
    let exact = std::env::args().nth(1).as_deref() == Some("on");
    let number: serde_json::Value = serde_json::from_str("0.10000000000000001").unwrap();
    assert_eq!(number.to_string(), if exact { "0.10000000000000001" } else { "0.1" });
    let tiny: serde_json::Value = serde_json::from_str("1e-400").unwrap();
    assert_eq!(tiny.to_string(), if exact { "1e-400" } else { "0.0" });
    assert_eq!(serde_json::from_str::<serde_json::Value>("1e400").is_ok(), exact);
}
'''


def main() -> None:
    environment = os.environ.copy()
    # Reuse compiled dependencies, but never share the workspace's feature graph.
    environment["CARGO_TARGET_DIR"] = str(REPOSITORY / "target" / "feature-consumers")
    with tempfile.TemporaryDirectory(prefix="jsoncompat-features-") as directory:
        consumer = Path(directory)
        (consumer / "src").mkdir()
        (consumer / "src" / "main.rs").write_text(PROBE, encoding="utf-8")
        for package, path in PACKAGES.items():
            for mode in ("default", "no-default", "precision"):
                options = {
                    "default": "",
                    "no-default": ", default-features = false",
                    "precision": ', default-features = false, features = ["arbitrary-precision"]',
                }[mode]
                dependency_path = json.dumps(str(REPOSITORY / path))
                (consumer / "Cargo.toml").write_text(
                    '[package]\nname = "precision-consumer"\nversion = "0.0.0"\nedition = "2024"\n'
                    '[workspace]\n[dependencies]\nserde_json = "1.0.149"\n'
                    f'{package} = {{ path = {dependency_path}{options} }}\n',
                    encoding="utf-8",
                )
                shutil.copyfile(REPOSITORY / "Cargo.lock", consumer / "Cargo.lock")
                metadata = json.loads(subprocess.check_output(
                    ["cargo", "metadata", "--offline", "--format-version=1"],
                    cwd=consumer, env=environment, text=True,
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
                # Binding crates are cdylibs; their runtime suites exercise the
                # exports. Here their independent dependency graphs are checked.
                if package not in ("jsoncompat_py", "jsoncompat_wasm"):
                    subprocess.run(
                        ["cargo", "run", "--quiet", "--offline", "--locked", "--",
                         "on" if mode == "precision" else "off"],
                        cwd=consumer, env=environment, check=True,
                    )
                print(f"{package}: {mode} passed", flush=True)


if __name__ == "__main__":
    main()
