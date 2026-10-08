"""Build and exercise an sdist installation without importing the checkout."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import venv


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path, help="source distribution to install")
    archive: Path = parser.parse_args().archive.resolve()
    if not archive.is_file():
        parser.error(f"source distribution does not exist: {archive}")
    if not archive.name.endswith(".tar.gz"):
        parser.error("expected a .tar.gz source distribution, not a wheel")

    tests = Path(__file__).resolve().parent
    repository = tests.parents[1]
    environment = os.environ.copy()
    # Build fresh native artifacts and keep local Python modules out of the test.
    environment.pop("CARGO_TARGET_DIR", None)
    environment.pop("PYTHONPATH", None)
    environment.pop("VIRTUAL_ENV", None)

    with tempfile.TemporaryDirectory(prefix="jsoncompat-sdist-") as directory:
        root = Path(directory)
        source = root / archive.name
        shutil.copy2(archive, source)
        shutil.copy2(tests / "test_installed.py", root / "test_installed.py")
        for name in ("dataclasses", "stamp"):
            destination = root / "examples" / name
            destination.mkdir(parents=True)
            for module in (repository / "examples" / name).glob("*.py"):
                shutil.copy2(module, destination / module.name)

        virtualenv = root / "venv"
        venv.EnvBuilder(with_pip=True).create(virtualenv)
        python = virtualenv / (
            "Scripts/python.exe" if os.name == "nt" else "bin/python"
        )
        subprocess.run(
            [
                str(python),
                "-I",
                "-m",
                "pip",
                "--isolated",
                "install",
                "--no-cache-dir",
                f"{source}[msgpack,yaml]",
            ],
            cwd=root,
            env=environment,
            check=True,
        )
        subprocess.run(
            [str(python), "-I", str(root / "test_installed.py"), "-v"],
            cwd=root,
            env=environment,
            check=True,
        )


if __name__ == "__main__":
    main()
