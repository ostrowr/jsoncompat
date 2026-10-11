"""Test a built wheel with no HTTP extras, then each integration in isolation."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import tempfile
import venv
from pathlib import Path


def check_http_extras(wheel: Path) -> None:
    environment = os.environ.copy()
    for name in ("PYTHONPATH", "VIRTUAL_ENV", "JSONCOMPAT_NATIVE_PROFILE"):
        environment.pop(name, None)
    with tempfile.TemporaryDirectory(prefix="jsoncompat-http-extras-") as directory:
        root = Path(directory)
        source = root / wheel.name
        shutil.copy2(wheel, source)
        test = root / "test_http_extra.py"
        shutil.copy2(Path(__file__).with_name(test.name), test)
        for extra in ("core", "aiohttp", "httpx", "requests"):
            virtualenv = root / extra
            venv.EnvBuilder(with_pip=True).create(virtualenv)
            python = virtualenv / (
                "Scripts/python.exe" if os.name == "nt" else "bin/python"
            )
            requirement = str(source) if extra == "core" else f"{source}[{extra}]"
            subprocess.run(
                [str(python), "-I", "-m", "pip", "--isolated", "install", requirement],
                cwd=root,
                env=environment,
                check=True,
            )
            subprocess.run(
                [str(python), "-I", str(test), extra],
                cwd=root,
                env=environment,
                check=True,
            )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wheel", type=Path)
    wheel: Path = parser.parse_args().wheel.resolve()
    if not wheel.is_file() or wheel.suffix != ".whl":
        parser.error("expected a built .whl file for the active Python interpreter")
    check_http_extras(wheel)
