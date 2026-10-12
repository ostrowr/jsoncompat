"""Opt-in GitHub OpenAPI scale experiment; see benchmarks/github-openapi.md.

Downloads a pinned upstream document. Every projection is saved with an exclusion
log; schema constraints are never simplified to make a measurement succeed.
Run with the repository's Python 3.12 environment after building release binaries.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import resource
import shutil
import subprocess
import sys
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
REVISION = "58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1"
URL = f"https://raw.githubusercontent.com/github/rest-api-description/{REVISION}/descriptions-next/api.github.com/api.github.com.json"
SHA256 = "0db1a3dee41ed680628a2a3381a114da7358ac8aa3f7b9276c9198608ca6a1fe"
METHODS = {"get", "post", "put", "patch", "delete", "head", "options", "trace"}
TY_VERSION = "0.0.86"


def save(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n")


def prepare(directory: Path, case: str) -> tuple[Path, list[str]]:
    source = directory / "github.json"
    if not source.exists():
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read()
        if hashlib.sha256(data).hexdigest() != SHA256:
            raise ValueError("download does not match the pinned GitHub document")
        source.write_bytes(data)
    if hashlib.sha256(source.read_bytes()).hexdigest() != SHA256:
        raise ValueError("cached GitHub document differs from the pinned source")
    original = json.loads(source.read_text())
    spec = copy.deepcopy(original)
    exclusions: list[dict[str, object]] = []
    flags = [] if case == "models" else ["--api", "GitHub"]
    if case == "models":
        spec["paths"] = {}
        exclusions.append({"reason": "model-only run; no outbound operations"})
    elif case != "original":
        shapes: dict[str, str] = {}

        def resolve(value):
            if "$ref" not in value:
                return value
            result = spec
            for part in value["$ref"][2:].split("/"):
                result = result[part.replace("~1", "/").replace("~0", "~")]
            return resolve(result)

        for path, item in list(spec["paths"].items()):
            shape = re.sub(r"\{[^}]+\}", "{}", path)
            duplicate = shapes.get(shape)
            shapes.setdefault(shape, path)
            for method, operation in list(item.items()):
                if method not in METHODS:
                    continue
                media = set(resolve(operation.get("requestBody", {})).get("content", {}))
                for response in operation["responses"].values():
                    media.update(resolve(response).get("content", {}))
                unsupported = sorted(m for m in media if not (
                    m in {"application/json", "text/plain"}
                    or m.startswith("application/") and m.endswith("+json")
                ))
                reason = (
                    f"duplicate path shape: {duplicate}" if duplicate else
                    f"unsupported media: {unsupported}" if unsupported else
                    "outside issues/ namespace" if case == "issues" and not
                    operation.get("operationId", "").startswith("issues/") else None
                )
                if reason:
                    exclusions.append({"path": path, "method": method,
                                       "operationId": operation.get("operationId"), "reason": reason})
                    del item[method]
            if not METHODS.intersection(item):
                del spec["paths"][path]
        # These are inbound contracts and documentation examples, respectively.
        # Leave request/response schemas and their references unchanged.
        for owner, key in [(spec, "webhooks"), (spec["components"], "examples")]:
            removed = owner.pop(key, {})
            exclusions.append({"collection": key, "count": len(removed),
                               "reason": "unsupported by outbound operation lowering"})
        if case == "issues":
            # Select operation roots and their transitive model dependencies.
            flags += ["--component", "basic-error"]
    path = directory / f"{case}.json"
    save(path, spec)
    save(directory / f"{case}-projection.json", {
        "url": URL, "sha256": SHA256, "case": case,
        "operations": sum(m in METHODS for item in spec["paths"].values() for m in item),
        "component_schemas": len(spec["components"]["schemas"]), "exclusions": exclusions,
    })
    return path, flags


def measurement_worker(prefix: Path, timeout: float, command: list[str]) -> None:
    start = time.perf_counter()
    with prefix.with_suffix(".stdout").open("w") as out, prefix.with_suffix(".stderr").open("w") as err:
        # Own the entire group so a timeout also stops uv/node grandchildren.
        process = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
        try:
            code: int | str = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            import signal
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            code = "timeout"
    rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    save(prefix.with_suffix(".json"), {
        "command": command, "seconds": time.perf_counter() - start, "exit_code": code,
        "max_rss_bytes": rss if platform.system() == "Darwin" else rss * 1024,
    })


def measure(directory: Path, name: str, timeout: float, command: list[str]):
    prefix = directory / name
    subprocess.run([sys.executable, __file__, "_measure", str(prefix), str(timeout), *command], check=True)
    result = json.loads(prefix.with_suffix(".json").read_text())
    print(f"{name}: {result['seconds']:.3f}s; exit {result['exit_code']}", flush=True)
    return result


def inventory(package: Path) -> dict[str, object]:
    files = sorted((p.stat().st_size, str(p.relative_to(package)))
                   for p in package.rglob("*") if p.is_file() and "__pycache__" not in p.parts)
    sources = [item for item in files if item[1].endswith((".py", ".pyi"))]
    manifest = json.loads((package / ".jsoncompat-manifest.json").read_text())
    return {
        "files": len(files), "bytes": sum(size for size, _ in files),
        "python_files": len(sources), "python_bytes": sum(size for size, _ in sources),
        "largest_files": list(reversed(files[-10:])),
        "largest_python_file": sources[-1], "p95_python_bytes": sources[int(.95 * (len(sources) - 1))][0],
        "binding_groups": len(manifest["modules"]),
        "declarations": sum(p.startswith("_models/") and not Path(p).name.startswith("_")
                            for _, p in sources),
        "schema_storage_bytes": sum(size for size, p in sources if Path(p).name.startswith("_schema_")),
        "schema_chunk_files": sum(Path(p).name.startswith("_schema_") for _, p in sources),
    }


def negative_check(exit_code: int | str, stdout: str) -> dict[str, object]:
    result: dict[str, object] = {
        "exit_code": exit_code,
        "expected_errors_detected": False,
    }
    try:
        diagnostics = json.loads(stdout)
    except json.JSONDecodeError as error:
        result["diagnostic_json_error"] = str(error)
        return result
    if not isinstance(diagnostics, list):
        result["diagnostic_json_error"] = "expected a GitLab diagnostic array"
        return result
    if any(not isinstance(entry, dict) for entry in diagnostics):
        result["diagnostic_json_error"] = "expected diagnostic objects"
        return result
    rules = [entry.get("check_name") for entry in diagnostics]
    result["expected_errors_detected"] = (
        exit_code == 1 and all(isinstance(rule, str) for rule in rules)
        and set(rules) == {"invalid-argument-type", "invalid-return-type"}
    )
    return result


def configure_environment(directory: Path) -> None:
    env = os.environ
    env["PYTHONPATH"] = os.pathsep.join([str(ROOT / "pybindings"), str(directory)])
    env["JSONCOMPAT_NATIVE_PROFILE"] = "release"
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    for key in ["VIRTUAL_ENV", "UV_DEFAULT_INDEX", "UV_INDEX", "UV_INDEX_URL", "UV_EXTRA_INDEX_URL"]:
        env.pop(key, None)


def inspect(directory: Path, case: str, package: Path, timeout: float, repeats: int):
    name = package.name
    configure_environment(directory)
    save(directory / f"{case}-files.json", inventory(package))
    for cache in package.rglob("__pycache__"):
        shutil.rmtree(cache)
    load = ("[getattr(module, n) for n in module.__all__]" if case == "models"
            else "getattr(module, 'GitHubAPI')")
    source = ("import importlib,json,time; t=time.perf_counter(); "
              f"module=importlib.import_module({name!r}); root=time.perf_counter()-t; "
              f"t=time.perf_counter(); {load}; "
              "print(json.dumps(dict(package_seconds=root,load_seconds=time.perf_counter()-t)))")
    for phase in ["source", "bytecode"]:
        if phase == "bytecode":
            measure(directory, f"{case}-compileall", timeout,
                    [sys.executable, "-m", "compileall", "-q", str(package)])
        for repeat in range(repeats):
            measure(directory, f"{case}-import-{phase}-{repeat}", timeout, [sys.executable, "-c", source])
    typecheck(directory, case, package, timeout)


def typecheck(directory: Path, case: str, package: Path, timeout: float) -> None:
    name = package.name
    configure_environment(directory)
    consumer = directory / f"{case}_consumer.py"
    if case == "models":
        consumer.write_text(f"""from typing import assert_type
from {name} import SimpleUser
from {name}.models import SimpleUser as BarrelUser
def check(user: BarrelUser) -> SimpleUser:
    assert_type(user.login, str)
    assert_type(user.id, int)
    assert_type(SimpleUser.deserialize(user.serialize()), SimpleUser)
    return user
""")
    else:
        consumer.write_text(f"""from typing import assert_type
from {name} import GitHubClient, GitHubAsyncClient, IssuesGetRequest, IssuesGet200
def get_issue(client: GitHubClient) -> IssuesGet200:
    result = client.request(IssuesGetRequest(owner="octocat", repo="Hello-World", issue_number=1))
    response = result.unwrap()
    assert_type(response, IssuesGet200)
    assert_type(response.body.title, str)
    return response
async def get_issue_async(client: GitHubAsyncClient) -> IssuesGet200:
    return (await client.request(IssuesGetRequest(owner="octocat", repo="Hello-World", issue_number=1))).unwrap()
""")
    launcher = [
        "uv", "run", "--no-config", "--project", str(ROOT / "pybindings"), "--all-extras", "--locked",
        "--with", f"ty=={TY_VERSION}", "ty",
    ]
    command = [
        *launcher, "check", "--project", str(directory), "--python-version", "3.12",
        "--extra-search-path", str(ROOT / "pybindings"), "--extra-search-path", str(directory),
        "--error", "all", "--output-format", "gitlab",
    ]
    manifest = json.loads((package / ".jsoncompat-manifest.json").read_text())
    save(directory / f"{case}-ty-environment.json", {
        "typechecker": "ty", "version": TY_VERSION,
        "version_output": subprocess.check_output([*launcher, "--version"], text=True).strip(),
        "platform": platform.platform(), "python": sys.version,
        "python_version_target": "3.12", "rules": "--error all", "command": command,
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "working_tree": subprocess.check_output(["git", "status", "--short"], cwd=ROOT, text=True),
        "package": str(package), "package_compiler": manifest["compiler"],
        "node_options": os.environ.get("NODE_OPTIONS"),
    })
    measure(directory, f"{case}-ty", timeout, [*command, str(package), str(consumer)])
    if case != "models":
        invalid = directory / f"{case}_invalid.py"
        invalid.write_text(f"""from {name} import GitHubClient, IssuesGetRequest
def bad(client: GitHubClient) -> int:
    request = IssuesGetRequest(owner="octocat", repo="Hello-World", issue_number="wrong")
    return client.request(request).unwrap().body.title
""")
        result = measure(directory, f"{case}-ty-negative", timeout, [*command, str(invalid)])
        save(directory / f"{case}-ty-negative-check.json", {
            **negative_check(result["exit_code"],
                             (directory / f"{case}-ty-negative.stdout").read_text()),
            "stderr_file": f"{case}-ty-negative.stderr",
        })


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="fresh experiment directory")
    parser.add_argument("--case", choices=["original", "models", "client", "issues"], required=True)
    parser.add_argument("--timeout", type=float, default=900)
    parser.add_argument("--repeats", type=int, default=3)
    phase = parser.add_mutually_exclusive_group()
    phase.add_argument("--prepare-only", action="store_true")
    phase.add_argument("--inspect-only", action="store_true", help="reuse output; generation timing is not measured")
    phase.add_argument("--typecheck-only", action="store_true",
                       help="check an existing package with ty; preserve generation and import measurements")
    args = parser.parse_args()
    if args.timeout <= 0 or args.repeats < 1:
        parser.error("timeout and repeats must be positive")
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    package = directory / f"github_{args.case}"
    if args.typecheck_only:
        if not package.is_dir():
            parser.error("--typecheck-only requires an existing generated package")
        typecheck(directory, args.case, package, args.timeout)
        return
    spec, flags = prepare(directory, args.case)
    if args.prepare_only:
        return
    save(directory / f"{args.case}-environment.json", {
        "platform": platform.platform(), "python": sys.version,
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "working_tree": subprocess.check_output(["git", "status", "--short"], cwd=ROOT, text=True),
        "generator_sha256": hashlib.sha256((ROOT / "target/release/jsoncompat").read_bytes()).hexdigest(),
        "native_profile": "release", "repeats": args.repeats,
        "node_options": os.environ.get("NODE_OPTIONS"),
    })
    if not args.inspect_only:
        if package.exists():
            parser.error("use a fresh output directory for cold generation, or --inspect-only")
        result = measure(directory, f"{args.case}-generation", args.timeout, [
            str(ROOT / "target/release/jsoncompat"), "codegen", "--openapi", str(spec),
            "--output", str(package), *flags,
        ])
        if result["exit_code"] != 0:
            return
    inspect(directory, args.case, package, args.timeout, args.repeats)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "_measure":
        measurement_worker(Path(sys.argv[2]), float(sys.argv[3]), sys.argv[4:])
    else:
        main()
