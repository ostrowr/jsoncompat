"""Benchmark generated jsoncompat and Pydantic models for every schema fixture.

The jsoncompat side consumes the checked-in generated snapshots under
``tests/fixtures/dataclasses``. The Pydantic side is generated from the exact
same schemas with the pinned ``datamodel-code-generator`` dependency. Generated
Pydantic modules and detailed results are written below ``target`` so the
benchmark never makes generated build artifacts part of the source tree.

Every schema is represented in the result manifest, including explicit
jsoncompat codegen failures, Pydantic codegen/import failures, and schemas for
which the generated validators disagree or no shared valid value could be
found. Pydantic acceptance is screened against the jsoncompat validator using
fixture tests and deterministic mutations before timing. This prevents a
faster-looking result from silently narrowing the Pydantic comparison corpus
or dropping validation work. Independently of Pydantic compatibility, every
fixture with a generated jsoncompat model and a reproducible valid sample is
timed end to end from JSON through the generated dataclass and back to JSON.
"""

from __future__ import annotations

import argparse
import gc
import hashlib
import importlib.util
import json
import math
import platform
import statistics
import sys
import time
import warnings
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass
from importlib.metadata import version as package_version
from pathlib import Path
from types import ModuleType
from typing import Any, Literal, TypeAlias, TypedDict, cast, get_args

import pydantic
from datamodel_code_generator import (
    InputFileType,
    LiteralType,
    generate,
)
from datamodel_code_generator.enums import DataModelType, StrictTypes
from datamodel_code_generator.format import Formatter, PythonVersion
from pydantic import TypeAdapter

import jsoncompat

JsonScalar: TypeAlias = None | bool | int | float | str
JsonValue: TypeAlias = JsonScalar | list["JsonValue"] | dict[str, "JsonValue"]

REPO_ROOT = Path(__file__).resolve().parents[1]
FIXTURE_ROOT = REPO_ROOT / "tests" / "fixtures"
FUZZ_ROOT = FIXTURE_ROOT / "fuzz"
BACKCOMPAT_ROOT = FIXTURE_ROOT / "backcompat"
JSONCOMPAT_MODEL_ROOT = FIXTURE_ROOT / "dataclasses"
DEFAULT_OUTPUT_ROOT = REPO_ROOT / "target" / "python-fixture-benchmark"
CHECKED_SAMPLE_CACHE = REPO_ROOT / "pybindings" / "bench_fixture_samples.json"
RUNTIME_UNSUPPORTED_MANIFEST = JSONCOMPAT_MODEL_ROOT / "runtime_unsupported.json"
UNSATISFIABLE_MANIFEST = JSONCOMPAT_MODEL_ROOT / "unsatisfiable.json"

PYDANTIC_BASE_CLASS = "fixture_benchmark_support.StrictBaseModel"
PYDANTIC_ROOT_NAME = "GeneratedSchema"
GENERATOR_CONFIGURATION = "pydantic-v2-strict-literals-v1"

Status = Literal[
    "benchmarked",
    "jsoncompat_unsupported",
    "pydantic_codegen_error",
    "pydantic_import_error",
    "jsoncompat_import_error",
    "jsoncompat_validation_unsupported",
    "unsatisfiable",
    "pydantic_semantic_mismatch",
    "no_shared_value",
]


@dataclass(frozen=True, slots=True)
class FixtureCase:
    """One standalone JSON Schema extracted from the fixture corpus."""

    case_id: str
    schema: JsonValue
    source_path: Path
    schema_index: int | None
    jsoncompat_model_path: Path | None
    jsoncompat_error_path: Path | None
    fixture_candidates: tuple[JsonValue, ...]
    validation_candidates: tuple[JsonValue, ...]

    @property
    def schema_json(self) -> str:
        return canonical_json(self.schema)

    @property
    def schema_digest(self) -> str:
        return hashlib.sha256(self.schema_json.encode()).hexdigest()


@dataclass(frozen=True, slots=True)
class PreparedValue:
    """A value accepted and emitted identically by both generated models."""

    value: JsonValue
    wire: str
    source: str
    pydantic_python_compatible: bool


@dataclass(frozen=True, slots=True)
class CachedCandidate:
    """A present cached sample, including JSON null."""

    value: JsonValue


class UnsatisfiableFixture(TypedDict):
    reason: str
    schema_digest: str


@dataclass(frozen=True, slots=True)
class Comparison:
    """One jsoncompat operation and its closest Pydantic baseline."""

    name: str
    jsoncompat_key: str
    pydantic_key: str
    requires_python_value: bool = False


COMPARISONS = (
    Comparison(
        "value -> model (checked)",
        "jsoncompat.from_value.checked",
        "pydantic.validate_python",
        requires_python_value=True,
    ),
    Comparison(
        "value -> model (trusted)",
        "jsoncompat.from_value.trusted",
        "pydantic.validate_python",
        requires_python_value=True,
    ),
    Comparison(
        "model -> value (checked)",
        "jsoncompat.to_value.checked",
        "pydantic.dump_python",
    ),
    Comparison(
        "model -> value (trusted)",
        "jsoncompat.to_value.trusted",
        "pydantic.dump_python",
    ),
    Comparison(
        "model -> JSON (checked)",
        "jsoncompat.serialize.checked",
        "pydantic.dump_json",
    ),
    Comparison(
        "model -> JSON (trusted)",
        "jsoncompat.serialize.trusted",
        "pydantic.dump_json",
    ),
    Comparison(
        "JSON -> model (checked)",
        "jsoncompat.deserialize.checked",
        "pydantic.validate_json",
    ),
    Comparison(
        "JSON -> model (trusted)",
        "jsoncompat.deserialize.trusted",
        "pydantic.validate_json",
    ),
    Comparison(
        "JSON -> model -> JSON (checked)",
        "jsoncompat.roundtrip.checked",
        "pydantic.roundtrip",
    ),
    Comparison(
        "JSON -> model -> JSON (trusted)",
        "jsoncompat.roundtrip.trusted",
        "pydantic.roundtrip",
    ),
)

JSONCOMPAT_END_TO_END_OPERATIONS = (
    ("JSON -> dataclass -> JSON (checked)", "jsoncompat.roundtrip.checked"),
    ("JSON -> dataclass -> JSON (trusted)", "jsoncompat.roundtrip.trusted"),
)

GENERIC_SEMANTIC_PROBES: tuple[JsonValue, ...] = (
    None,
    False,
    True,
    -1,
    0,
    1,
    1.5,
    "",
    "x",
    [],
    [0],
    [1, 2, 3],
    {},
    {"x": 1},
)


def canonical_json(value: JsonValue) -> str:
    """Return a deterministic, strict JSON representation."""

    return json.dumps(
        value,
        allow_nan=False,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    )


def load_json(path: Path) -> JsonValue:
    return cast(JsonValue, json.loads(path.read_text()))


def snapshot_paths(relative_base: Path) -> tuple[Path | None, Path | None]:
    model_path = (JSONCOMPAT_MODEL_ROOT / relative_base).with_suffix(".py")
    error_path = (JSONCOMPAT_MODEL_ROOT / relative_base).with_suffix(".error.txt")
    model_exists = model_path.is_file()
    error_exists = error_path.is_file()
    if model_exists == error_exists:
        raise RuntimeError(
            f"expected exactly one jsoncompat snapshot for {relative_base}, "
            f"found model={model_exists} error={error_exists}"
        )
    return (
        model_path if model_exists else None,
        error_path if error_exists else None,
    )


def backcompat_candidates(case_dir: Path, side: str) -> tuple[JsonValue, ...]:
    examples_path = case_dir / "examples.json"
    if not examples_path.is_file():
        return ()
    examples = load_json(examples_path)
    if not isinstance(examples, dict):
        raise RuntimeError(f"expected object in {examples_path}")
    values: list[JsonValue] = []
    for key in ("both", f"{side}_only"):
        candidates = examples.get(key, [])
        if not isinstance(candidates, list):
            raise RuntimeError(f"expected array at {examples_path}:{key}")
        values.extend(candidates)
    return tuple(values)


def iter_backcompat_cases() -> Iterator[FixtureCase]:
    for case_dir in sorted(path for path in BACKCOMPAT_ROOT.iterdir() if path.is_dir()):
        for side in ("old", "new"):
            source_path = case_dir / f"{side}.json"
            relative_base = Path("backcompat") / case_dir.name / side
            model_path, error_path = snapshot_paths(relative_base)
            candidates = backcompat_candidates(case_dir, side)
            yield FixtureCase(
                case_id=relative_base.as_posix(),
                schema=load_json(source_path),
                source_path=source_path,
                schema_index=None,
                jsoncompat_model_path=model_path,
                jsoncompat_error_path=error_path,
                fixture_candidates=candidates,
                validation_candidates=candidates,
            )


def embedded_fuzz_cases(
    path: Path,
) -> list[tuple[JsonValue, tuple[JsonValue, ...], tuple[JsonValue, ...]]]:
    root = load_json(path)
    if not isinstance(root, list):
        return [(root, (), ())]

    result: list[tuple[JsonValue, tuple[JsonValue, ...], tuple[JsonValue, ...]]] = []
    for item in root:
        if not isinstance(item, dict) or "schema" not in item:
            continue
        raw_tests = item.get("tests", [])
        if not isinstance(raw_tests, list):
            raw_tests = []
        candidates = tuple(
            test["data"]
            for test in raw_tests
            if isinstance(test, dict) and test.get("valid") is True and "data" in test
        )
        validation_candidates = tuple(
            test["data"]
            for test in raw_tests
            if isinstance(test, dict) and "data" in test
        )
        result.append((item["schema"], candidates, validation_candidates))
    return result


def iter_fuzz_cases() -> Iterator[FixtureCase]:
    for source_path in sorted(FUZZ_ROOT.rglob("*.json")):
        relative_path = source_path.relative_to(FUZZ_ROOT)
        snapshot_dir = Path("fuzz") / relative_path.with_suffix("")
        for index, (schema, candidates, validation_candidates) in enumerate(
            embedded_fuzz_cases(source_path)
        ):
            relative_base = snapshot_dir / f"{index:03}"
            model_path, error_path = snapshot_paths(relative_base)
            yield FixtureCase(
                case_id=relative_base.as_posix(),
                schema=schema,
                source_path=source_path,
                schema_index=index,
                jsoncompat_model_path=model_path,
                jsoncompat_error_path=error_path,
                fixture_candidates=candidates,
                validation_candidates=validation_candidates,
            )


def fixture_cases() -> list[FixtureCase]:
    cases = sorted(
        [*iter_backcompat_cases(), *iter_fuzz_cases()],
        key=lambda case: case.case_id,
    )
    case_ids = [case.case_id for case in cases]
    if len(case_ids) != len(set(case_ids)):
        raise RuntimeError("fixture case identifiers are not unique")
    return cases


def artifact_path(root: Path, case_id: str, suffix: str) -> Path:
    path = root.joinpath(*case_id.split("/"))
    return path.with_suffix(suffix)


def pydantic_source(case: FixtureCase) -> str:
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        generated = generate(
            cast(Any, case.schema),
            input_filename=f"{case.case_id}.json",
            input_file_type=InputFileType.JsonSchema,
            output_model_type=DataModelType.PydanticV2BaseModel,
            base_class=PYDANTIC_BASE_CLASS,
            class_name=PYDANTIC_ROOT_NAME,
            target_python_version=PythonVersion.PY_312,
            strict_types=list(StrictTypes),
            enum_field_as_literal=LiteralType.All,
            disable_timestamp=True,
            field_constraints=True,
            use_annotated=True,
            use_standard_collections=True,
            use_union_operator=True,
            formatters=[Formatter.BUILTIN],
        )
    if not isinstance(generated, str):
        raise RuntimeError(
            f"generator returned {type(generated).__name__}, expected source"
        )
    return f"{generated.rstrip()}\n\nPYDANTIC_MODEL = {PYDANTIC_ROOT_NAME}\n"


def generate_pydantic_models(
    cases: Sequence[FixtureCase],
    models_root: Path,
    *,
    reuse_models: bool,
) -> dict[str, dict[str, str]]:
    """Generate every Pydantic peer and return per-case generation metadata."""

    outcomes: dict[str, dict[str, str]] = {}
    generator_version = package_version("datamodel-code-generator")
    for position, case in enumerate(cases, start=1):
        model_path = artifact_path(models_root, case.case_id, ".py")
        digest_path = artifact_path(models_root, case.case_id, ".sha256")
        error_path = artifact_path(models_root, case.case_id, ".error.txt")
        generation_digest = hashlib.sha256(
            (
                f"{generator_version}\0{GENERATOR_CONFIGURATION}\0{case.schema_json}"
            ).encode()
        ).hexdigest()

        if (
            reuse_models
            and model_path.is_file()
            and digest_path.is_file()
            and digest_path.read_text().strip() == generation_digest
        ):
            outcomes[case.case_id] = {
                "status": "generated",
                "path": str(model_path.relative_to(REPO_ROOT)),
                "cache": "hit",
            }
            continue

        model_path.parent.mkdir(parents=True, exist_ok=True)
        try:
            source = pydantic_source(case)
            compile(source, str(model_path), "exec")
            model_path.write_text(source)
            digest_path.write_text(f"{generation_digest}\n")
            error_path.unlink(missing_ok=True)
            outcomes[case.case_id] = {
                "status": "generated",
                "path": str(model_path.relative_to(REPO_ROOT)),
                "cache": "miss",
            }
        except Exception as error:  # noqa: BLE001 - the manifest records tool failures
            model_path.unlink(missing_ok=True)
            digest_path.unlink(missing_ok=True)
            error_path.parent.mkdir(parents=True, exist_ok=True)
            error_path.write_text(f"{type(error).__name__}: {error}\n")
            outcomes[case.case_id] = {
                "status": "error",
                "path": str(error_path.relative_to(REPO_ROOT)),
                "error": f"{type(error).__name__}: {error}",
            }

        if position % 50 == 0 or position == len(cases):
            print(f"generated Pydantic peers: {position}/{len(cases)}", flush=True)
    return outcomes


def import_module(
    path: Path, case_id: str, implementation: str
) -> tuple[str, ModuleType]:
    digest = hashlib.sha256(f"{implementation}\0{case_id}".encode()).hexdigest()[:16]
    module_name = f"_jsoncompat_fixture_bench_{implementation}_{digest}"
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not create import spec for {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        sys.modules.pop(module_name, None)
        raise
    return module_name, module


def model_from_module(module: ModuleType, attribute: str) -> Any:
    try:
        return getattr(module, attribute)
    except AttributeError as error:
        raise RuntimeError(f"generated module does not export {attribute}") from error


def unproductive_recursive_root(module: ModuleType) -> str | None:
    """Detect direct ``X = RootModel[X | ...]`` recursion before validation.

    Such generated validators recurse without consuming any input and can
    overflow the native Pydantic validator stack instead of raising a normal
    validation error. Productive recursion through a field or container is
    intentionally allowed.
    """

    for name, value in vars(module).items():
        if not isinstance(value, type) or not issubclass(value, pydantic.RootModel):
            continue
        root_field = value.model_fields.get("root")
        if root_field is None:
            continue
        annotation = root_field.annotation
        if annotation is value or value in get_args(annotation):
            return name
    return None


def dump_pydantic(adapter: TypeAdapter[Any], instance: Any) -> JsonValue:
    return cast(
        JsonValue,
        adapter.dump_python(
            instance,
            mode="json",
            by_alias=True,
            exclude_unset=True,
        ),
    )


def dump_pydantic_json(adapter: TypeAdapter[Any], instance: Any) -> str:
    return adapter.dump_json(
        instance,
        by_alias=True,
        exclude_unset=True,
    ).decode()


def mutation_probes(value: JsonValue) -> Iterator[JsonValue]:
    """Produce deterministic near-neighbor values for semantic screening."""

    if isinstance(value, dict):
        keys = sorted(value)[:8]
        replacements: tuple[JsonValue, ...] = (None, False, 0, "", [], {})
        for key in keys:
            removed = dict(value)
            removed.pop(key)
            yield removed
            for replacement in replacements:
                replaced = dict(value)
                replaced[key] = replacement
                yield replaced
        extra = dict(value)
        extra["__jsoncompat_unexpected_probe__"] = None
        yield extra
    elif isinstance(value, list):
        yield []
        yield [None]
        yield [*value, None]
        if value:
            yield value[:1]
            replaced = list(value)
            replaced[0] = None
            yield replaced
    elif isinstance(value, str):
        yield ""
        yield f"{value}x"
        yield 0
    elif isinstance(value, bool):
        yield 0
        yield None
    elif isinstance(value, (int, float)):
        yield -1
        yield 0
        yield 1
        yield 1.5
        yield "1"
    else:
        yield 0


def semantic_probe_values(case: FixtureCase) -> Iterator[JsonValue]:
    seen: set[str] = set()
    seeds = (*GENERIC_SEMANTIC_PROBES, *case.validation_candidates)
    for candidate in seeds:
        for probe in (candidate, *mutation_probes(candidate)):
            try:
                fingerprint = canonical_json(probe)
            except (TypeError, ValueError):
                continue
            if fingerprint in seen:
                continue
            seen.add(fingerprint)
            yield probe


def semantic_mismatches(
    case: FixtureCase,
    pydantic_adapter: TypeAdapter[Any],
) -> tuple[int, int, list[dict[str, Any]]]:
    """Compare Pydantic acceptance with jsoncompat's schema validator oracle."""

    validator = jsoncompat.validator_for(case.schema_json)
    probe_count = 0
    mismatch_count = 0
    examples: list[dict[str, Any]] = []
    for value in semantic_probe_values(case):
        probe_count += 1
        wire = canonical_json(value)
        expected = validator.is_valid_json(wire)
        pydantic_error: str | None = None
        try:
            pydantic_adapter.validate_json(wire)
            actual = True
        except Exception as error:  # noqa: BLE001 - generated validators vary
            actual = False
            pydantic_error = f"{type(error).__name__}: {error}"
        if actual == expected:
            continue
        mismatch_count += 1
        if len(examples) < 5:
            examples.append(
                {
                    "value": value,
                    "jsoncompat_valid": expected,
                    "pydantic_valid": actual,
                    "pydantic_error": pydantic_error,
                }
            )
    return probe_count, mismatch_count, examples


def cached_candidate(
    sample_cache: Mapping[str, Any], case: FixtureCase
) -> CachedCandidate | None:
    raw_value = sample_cache.get(case.case_id)
    if not isinstance(raw_value, dict):
        return None
    raw = cast(Mapping[str, object], raw_value)
    if raw.get("schema_digest") != case.schema_digest or "value" not in raw:
        return None
    return CachedCandidate(cast(JsonValue, raw["value"]))


def candidate_stream(
    case: FixtureCase,
    sample_cache: Mapping[str, Any],
) -> Iterator[tuple[str, JsonValue]]:
    seen: set[str] = set()

    def emit(source: str, candidate: JsonValue) -> Iterator[tuple[str, JsonValue]]:
        try:
            fingerprint = canonical_json(candidate)
        except (TypeError, ValueError):
            return
        if fingerprint not in seen:
            seen.add(fingerprint)
            yield source, candidate

    cached = cached_candidate(sample_cache, case)
    if cached is not None:
        yield from emit("cache", cached.value)
    for candidate in case.fixture_candidates:
        yield from emit("fixture", candidate)


def prepare_jsoncompat_value(
    case: FixtureCase,
    jsoncompat_model: Any,
    sample_cache: Mapping[str, Any],
) -> tuple[PreparedValue | None, str | None]:
    """Find a stable JSON value for jsoncompat-only end-to-end timing."""

    last_error: str | None = None
    cached = cached_candidate(sample_cache, case)
    if cached is None:
        return None, "checked-in sample is missing or has a stale schema digest"
    for source, candidate in (("cache", cached.value),):
        try:
            candidate_wire = canonical_json(candidate)
            instance = jsoncompat_model.deserialize(candidate_wire)
            normalized_value = cast(
                JsonValue,
                instance.to_value(skip_validation=True),
            )
            normalized_wire = canonical_json(normalized_value)
            serialized = instance.serialize(skip_validation=True)
            if (
                canonical_json(cast(JsonValue, json.loads(serialized)))
                != normalized_wire
            ):
                last_error = "generated model emitted different value and JSON forms"
                continue

            # Defaults may normalize the original candidate. Use the stable output as
            # the timed input so every measured iteration exercises the same complete
            # JSON -> dataclass -> JSON path.
            roundtripped = jsoncompat_model.deserialize(normalized_wire)
            stable_wire = roundtripped.serialize(skip_validation=True)
            if (
                canonical_json(cast(JsonValue, json.loads(stable_wire)))
                != normalized_wire
            ):
                last_error = "generated model JSON round trip was not idempotent"
                continue

            return (
                PreparedValue(
                    value=normalized_value,
                    wire=normalized_wire,
                    source=source,
                    pydantic_python_compatible=False,
                ),
                None,
            )
        except (TypeError, ValueError) as error:
            last_error = f"{type(error).__name__}: {error}"
    return None, last_error


def prepare_value(
    case: FixtureCase,
    jsoncompat_model: Any,
    pydantic_adapter: TypeAdapter[Any],
    sample_cache: Mapping[str, Any],
) -> tuple[PreparedValue | None, str | None]:
    last_error: str | None = None
    for source, candidate in candidate_stream(case, sample_cache):
        try:
            wire = canonical_json(candidate)
            jsoncompat_instance = jsoncompat_model.deserialize(wire)
            pydantic_instance = pydantic_adapter.validate_json(wire)
            reference = cast(
                JsonValue,
                jsoncompat_instance.to_value(skip_validation=True),
            )
            pydantic_value = dump_pydantic(pydantic_adapter, pydantic_instance)
            if canonical_json(pydantic_value) != canonical_json(reference):
                last_error = "generated models emitted different JSON values"
                continue
            jsoncompat_wire = jsoncompat_instance.serialize(skip_validation=True)
            pydantic_wire = dump_pydantic_json(pydantic_adapter, pydantic_instance)
            if canonical_json(
                cast(JsonValue, json.loads(jsoncompat_wire))
            ) != canonical_json(cast(JsonValue, json.loads(pydantic_wire))):
                last_error = "generated models serialized different JSON values"
                continue

            normalized_wire = canonical_json(reference)
            pydantic_python_compatible = False
            try:
                python_instance = pydantic_adapter.validate_python(reference)
                pydantic_python_compatible = (
                    canonical_json(dump_pydantic(pydantic_adapter, python_instance))
                    == normalized_wire
                )
            except (TypeError, ValueError):
                pass

            return (
                PreparedValue(
                    value=reference,
                    wire=normalized_wire,
                    source=source,
                    pydantic_python_compatible=pydantic_python_compatible,
                ),
                None,
            )
        except (TypeError, ValueError) as error:
            last_error = f"{type(error).__name__}: {error}"
    return None, last_error


def benchmark_operation(
    callback: Callable[[], Any],
    *,
    iterations: int,
    repeats: int,
) -> float:
    for _ in range(min(iterations, 3)):
        callback()

    samples: list[float] = []
    gc_was_enabled = gc.isenabled()
    gc.disable()
    try:
        for _ in range(repeats):
            started = time.perf_counter_ns()
            for _ in range(iterations):
                callback()
            samples.append((time.perf_counter_ns() - started) / iterations)
    finally:
        if gc_was_enabled:
            gc.enable()
    return statistics.median(samples)


def benchmark_jsoncompat_end_to_end(
    jsoncompat_model: Any,
    prepared: PreparedValue,
    *,
    iterations: int,
    repeats: int,
) -> dict[str, float]:
    """Time the complete generated-dataclass JSON round trip."""

    wire = prepared.wire
    callbacks: dict[str, Callable[[], str]] = {
        "jsoncompat.roundtrip.checked": lambda: jsoncompat_model.deserialize(
            wire
        ).serialize(),
        "jsoncompat.roundtrip.trusted": lambda: jsoncompat_model.deserialize(
            wire,
            skip_validation=True,
        ).serialize(skip_validation=True),
    }
    for name, callback in callbacks.items():
        output = cast(JsonValue, json.loads(callback()))
        if canonical_json(output) != wire:
            raise RuntimeError(f"{name} changed the normalized fixture value")

    return {
        name: benchmark_operation(
            callback,
            iterations=iterations,
            repeats=repeats,
        )
        for name, callback in callbacks.items()
    }


def benchmark_case(
    jsoncompat_model: Any,
    pydantic_adapter: TypeAdapter[Any],
    prepared: PreparedValue,
    *,
    iterations: int,
    repeats: int,
) -> dict[str, float]:
    value = prepared.value
    wire = prepared.wire
    jsoncompat_instance = jsoncompat_model.deserialize(wire)
    pydantic_instance = pydantic_adapter.validate_json(wire)

    callbacks: dict[str, Callable[[], Any]] = {
        "jsoncompat.to_value.checked": lambda: jsoncompat_instance.to_value(),
        "jsoncompat.to_value.trusted": lambda: jsoncompat_instance.to_value(
            skip_validation=True
        ),
        "jsoncompat.serialize.checked": lambda: jsoncompat_instance.serialize(),
        "jsoncompat.serialize.trusted": lambda: jsoncompat_instance.serialize(
            skip_validation=True
        ),
        "jsoncompat.deserialize.checked": lambda: jsoncompat_model.deserialize(wire),
        "jsoncompat.deserialize.trusted": lambda: jsoncompat_model.deserialize(
            wire, skip_validation=True
        ),
        "pydantic.dump_python": lambda: pydantic_adapter.dump_python(
            pydantic_instance,
            mode="json",
            by_alias=True,
            exclude_unset=True,
        ),
        "pydantic.dump_json": lambda: pydantic_adapter.dump_json(
            pydantic_instance,
            by_alias=True,
            exclude_unset=True,
        ).decode(),
        "pydantic.validate_json": lambda: pydantic_adapter.validate_json(wire),
        "jsoncompat.roundtrip.checked": lambda: jsoncompat_model.deserialize(
            wire
        ).serialize(),
        "jsoncompat.roundtrip.trusted": lambda: jsoncompat_model.deserialize(
            wire,
            skip_validation=True,
        ).serialize(skip_validation=True),
        "pydantic.roundtrip": lambda: pydantic_adapter.dump_json(
            pydantic_adapter.validate_json(wire),
            by_alias=True,
            exclude_unset=True,
        ).decode(),
    }
    if prepared.pydantic_python_compatible:
        callbacks.update(
            {
                "jsoncompat.from_value.checked": lambda: jsoncompat_model.from_value(
                    value
                ),
                "jsoncompat.from_value.trusted": lambda: jsoncompat_model.from_value(
                    value, skip_validation=True
                ),
                "pydantic.validate_python": lambda: pydantic_adapter.validate_python(
                    value
                ),
            }
        )

    return {
        name: benchmark_operation(
            callback,
            iterations=iterations,
            repeats=repeats,
        )
        for name, callback in callbacks.items()
    }


def percentile(values: Sequence[float], fraction: float) -> float:
    if not values:
        raise ValueError("cannot calculate percentile of empty sequence")
    ordered = sorted(values)
    return ordered[round((len(ordered) - 1) * fraction)]


def geometric_mean(values: Sequence[float]) -> float:
    if not values:
        raise ValueError("cannot calculate geometric mean of empty sequence")
    return math.exp(statistics.fmean(math.log(value) for value in values))


def size_bucket(size: int) -> str:
    if size <= 128:
        return "small (<=128 B)"
    if size <= 1024:
        return "medium (129 B-1 KiB)"
    return "large (>1 KiB)"


def comparison_rows(
    records: Sequence[Mapping[str, Any]],
    comparison: Comparison,
) -> list[tuple[Mapping[str, Any], float, float, float]]:
    rows: list[tuple[Mapping[str, Any], float, float, float]] = []
    for record in records:
        timings = record.get("timings_ns")
        if not isinstance(timings, dict):
            continue
        typed_timings = cast(Mapping[str, object], timings)
        jsoncompat_ns = typed_timings.get(comparison.jsoncompat_key)
        pydantic_ns = typed_timings.get(comparison.pydantic_key)
        if not isinstance(jsoncompat_ns, (int, float)) or not isinstance(
            pydantic_ns, (int, float)
        ):
            continue
        rows.append(
            (
                record,
                float(jsoncompat_ns),
                float(pydantic_ns),
                float(jsoncompat_ns) / float(pydantic_ns),
            )
        )
    return rows


def summarize_comparison(
    records: Sequence[Mapping[str, Any]],
    comparison: Comparison,
) -> dict[str, Any]:
    rows = comparison_rows(records, comparison)
    ratios = [row[3] for row in rows]
    if not ratios:
        return {"name": comparison.name, "cases": 0}
    return {
        "name": comparison.name,
        "cases": len(rows),
        "jsoncompat_median_ns": statistics.median(row[1] for row in rows),
        "pydantic_median_ns": statistics.median(row[2] for row in rows),
        "median_ratio": statistics.median(ratios),
        "geometric_mean_ratio": geometric_mean(ratios),
        "p90_ratio": percentile(ratios, 0.90),
        "aggregate_ratio": sum(row[1] for row in rows) / sum(row[2] for row in rows),
        "jsoncompat_wins": sum(ratio < 1.0 for ratio in ratios),
    }


def summarize_jsoncompat_operation(
    records: Sequence[Mapping[str, Any]],
    name: str,
    timing_key: str,
) -> dict[str, Any]:
    timings: list[float] = []
    for record in records:
        record_timings = record.get("jsoncompat_end_to_end_timings_ns")
        if not isinstance(record_timings, dict):
            continue
        timing = cast(Mapping[str, object], record_timings).get(timing_key)
        if isinstance(timing, (int, float)):
            timings.append(float(timing))
    if not timings:
        return {"name": name, "timing_key": timing_key, "cases": 0}
    return {
        "name": name,
        "timing_key": timing_key,
        "cases": len(timings),
        "median_ns": statistics.median(timings),
        "p90_ns": percentile(timings, 0.90),
        "maximum_ns": max(timings),
    }


def print_summary(
    records: Sequence[Mapping[str, Any]],
    summaries: Sequence[Mapping[str, Any]],
    jsoncompat_end_to_end_summaries: Sequence[Mapping[str, Any]],
) -> None:
    statuses: dict[str, int] = {}
    for record in records:
        status = str(record["status"])
        statuses[status] = statuses.get(status, 0) + 1

    print("\nCoverage")
    print("--------")
    jsoncompat_generated = sum(
        record.get("jsoncompat_model_path") is not None for record in records
    )
    pydantic_generated = sum(
        isinstance(record.get("pydantic_generation"), dict)
        and record["pydantic_generation"].get("status") == "generated"
        for record in records
    )
    pydantic_imported = sum(
        isinstance(record.get("pydantic_import"), dict)
        and record["pydantic_import"].get("status") == "imported"
        for record in records
    )
    print(f"jsoncompat models generated   {jsoncompat_generated:5}/{len(records)}")
    print(f"Pydantic models generated     {pydantic_generated:5}/{len(records)}")
    print(f"Pydantic models imported      {pydantic_imported:5}/{len(records)}")
    jsoncompat_end_to_end = sum(
        record.get("jsoncompat_end_to_end_status") == "benchmarked"
        for record in records
    )
    print(
        f"jsoncompat JSON round trips   {jsoncompat_end_to_end:5}/"
        f"{jsoncompat_generated} generated"
    )
    end_to_end_statuses: dict[str, int] = {}
    for record in records:
        status = str(record.get("jsoncompat_end_to_end_status", "not_attempted"))
        end_to_end_statuses[status] = end_to_end_statuses.get(status, 0) + 1
    for status, count in sorted(end_to_end_statuses.items()):
        print(f"  E2E {status:22} {count:5}")
    for status, count in sorted(statuses.items()):
        print(f"{status:28} {count:5}")

    buckets: dict[str, int] = {}
    for record in records:
        bucket = record.get("jsoncompat_end_to_end_size_bucket")
        if isinstance(bucket, str):
            buckets[bucket] = buckets.get(bucket, 0) + 1
    if buckets:
        print("value sizes")
        for bucket, count in sorted(buckets.items()):
            print(f"  {bucket:26} {count:5}")

    print("\njsoncompat end-to-end JSON -> dataclass -> JSON")
    print("------------------------------------------------")
    print(
        f"{'operation':39} {'cases':>6} {'median us':>10} {'p90 us':>10} {'max us':>10}"
    )
    for summary in jsoncompat_end_to_end_summaries:
        cases = int(summary["cases"])
        if cases == 0:
            print(f"{str(summary['name']):39} {cases:6}")
            continue
        print(
            f"{str(summary['name']):39} {cases:6} "
            f"{float(summary['median_ns']) / 1_000:10.2f} "
            f"{float(summary['p90_ns']) / 1_000:10.2f} "
            f"{float(summary['maximum_ns']) / 1_000:10.2f}"
        )

    print("\nRuntime ratios (jsoncompat / Pydantic; lower is better)")
    print("------------------------------------------------------")
    print(
        f"{'operation':29} {'cases':>6} {'jc us':>8} {'pyd us':>8} "
        f"{'median':>8} {'p90':>8} {'aggregate':>10} {'wins':>8}"
    )
    for summary in summaries:
        cases = int(summary["cases"])
        if cases == 0:
            print(f"{str(summary['name']):29} {cases:6}")
            continue
        print(
            f"{str(summary['name']):29} {cases:6} "
            f"{float(summary['jsoncompat_median_ns']) / 1_000:8.2f} "
            f"{float(summary['pydantic_median_ns']) / 1_000:8.2f} "
            f"{float(summary['median_ratio']):8.2f} "
            f"{float(summary['p90_ratio']):8.2f} "
            f"{float(summary['aggregate_ratio']):10.2f} "
            f"{int(summary['jsoncompat_wins']):4}/{cases:<3}"
        )

    for comparison in (
        next(item for item in COMPARISONS if item.name == "JSON -> model (checked)"),
        next(item for item in COMPARISONS if item.name == "model -> JSON (trusted)"),
    ):
        rows = sorted(
            comparison_rows(records, comparison),
            key=lambda row: row[3],
            reverse=True,
        )[:10]
        print(f"\nLargest {comparison.name} ratios")
        print("-" * (8 + len(comparison.name)))
        for record, _, _, ratio in rows:
            print(
                f"{ratio:7.2f}x  {str(record['case_id']):70} "
                f"{int(record['value_bytes']):7} B"
            )


def load_sample_cache(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    raw = json.loads(path.read_text())
    if not isinstance(raw, dict):
        raise RuntimeError(f"expected object in {path}")
    return cast(dict[str, Any], raw)


def load_string_manifest(path: Path) -> dict[str, str]:
    raw: object = json.loads(path.read_text())
    if not isinstance(raw, dict):
        raise RuntimeError(f"expected non-empty string reasons in {path}")
    result: dict[str, str] = {}
    for case_id, reason in cast(dict[object, object], raw).items():
        if (
            not isinstance(case_id, str)
            or not isinstance(reason, str)
            or not reason.strip()
        ):
            raise RuntimeError(f"expected non-empty string reasons in {path}")
        result[case_id] = reason
    return result


def load_unsatisfiable_manifest(path: Path) -> dict[str, UnsatisfiableFixture]:
    raw: object = json.loads(path.read_text())
    if not isinstance(raw, dict):
        raise RuntimeError(f"expected object in {path}")
    result: dict[str, UnsatisfiableFixture] = {}
    for case_id, value in cast(dict[object, object], raw).items():
        if not isinstance(case_id, str) or not isinstance(value, dict):
            raise RuntimeError(f"invalid unsatisfiable classification in {path}")
        classification = cast(dict[object, object], value)
        reason = classification.get("reason")
        schema_digest = classification.get("schema_digest")
        if (
            set(classification) != {"reason", "schema_digest"}
            or not isinstance(reason, str)
            or not reason.strip()
            or not isinstance(schema_digest, str)
        ):
            raise RuntimeError(f"invalid unsatisfiable classification in {path}")
        result[case_id] = {
            "reason": reason,
            "schema_digest": schema_digest,
        }
    return result


def validate_fixture_classifications(
    cases: Sequence[FixtureCase],
    sample_cache: Mapping[str, Any],
    runtime_unsupported: Mapping[str, str],
    unsatisfiable: Mapping[str, UnsatisfiableFixture],
) -> None:
    """Require one deterministic sample or one exact classification per model."""

    by_id = {case.case_id: case for case in cases}
    known_ids = set(by_id)
    for name, case_ids in (
        ("sample cache", set(sample_cache)),
        ("runtime-unsupported manifest", set(runtime_unsupported)),
        ("unsatisfiable manifest", set(unsatisfiable)),
    ):
        unknown = sorted(case_ids - known_ids)
        if unknown:
            raise RuntimeError(f"{name} contains unknown fixture cases: {unknown}")

    overlap = sorted(set(runtime_unsupported) & set(unsatisfiable))
    if overlap:
        raise RuntimeError(
            f"fixtures cannot be both runtime-unsupported and unsatisfiable: {overlap}"
        )

    expected_samples: set[str] = set()
    for case in cases:
        classified = (
            case.case_id in runtime_unsupported or case.case_id in unsatisfiable
        )
        if case.jsoncompat_model_path is None:
            if classified or case.case_id in sample_cache:
                raise RuntimeError(
                    f"codegen-unsupported fixture {case.case_id} has a runtime "
                    "classification or sample"
                )
            continue
        if classified:
            if case.case_id in sample_cache:
                raise RuntimeError(
                    f"classified fixture {case.case_id} still has a runtime sample"
                )
            if case.case_id in unsatisfiable and case.fixture_candidates:
                raise RuntimeError(
                    f"unsatisfiable fixture {case.case_id} declares valid candidates"
                )
            if (
                case.case_id in unsatisfiable
                and unsatisfiable[case.case_id]["schema_digest"] != case.schema_digest
            ):
                raise RuntimeError(
                    f"unsatisfiable classification for {case.case_id} has a stale "
                    "schema digest"
                )
            continue

        expected_samples.add(case.case_id)
        raw_sample_value = sample_cache.get(case.case_id)
        if not isinstance(raw_sample_value, dict):
            raise RuntimeError(
                f"runtime-supported satisfiable fixture {case.case_id} has no "
                f"checked-in sample in {CHECKED_SAMPLE_CACHE}"
            )
        raw_sample = cast(Mapping[str, object], raw_sample_value)
        if "value" not in raw_sample:
            raise RuntimeError(
                f"runtime-supported satisfiable fixture {case.case_id} has no "
                f"checked-in sample in {CHECKED_SAMPLE_CACHE}"
            )
        if raw_sample.get("schema_digest") != case.schema_digest:
            raise RuntimeError(
                f"checked-in sample for {case.case_id} has a stale schema digest"
            )

    extra_samples = sorted(set(sample_cache) - expected_samples)
    if extra_samples:
        raise RuntimeError(
            "checked-in samples do not identify runtime-supported satisfiable "
            f"fixtures: {extra_samples}"
        )


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, allow_nan=False, indent=2, sort_keys=True) + "\n")


def positive_int(raw: str) -> int:
    value = int(raw)
    if value < 1:
        raise argparse.ArgumentTypeError("value must be at least 1")
    return value


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--iterations", type=positive_int, default=200)
    parser.add_argument("--repeats", type=positive_int, default=5)
    parser.add_argument(
        "--profile",
        default="manual",
        help="repeatable benchmark profile name recorded in the result manifest",
    )
    parser.add_argument("--limit", type=positive_int)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT_ROOT)
    parser.add_argument(
        "--results",
        type=Path,
        help=(
            "write the result manifest to this path while retaining generated "
            "Pydantic models under --output"
        ),
    )
    parser.add_argument(
        "--reuse-models",
        action="store_true",
        help="reuse Pydantic modules whose schema and generator digest match",
    )
    parser.add_argument(
        "--generate-only",
        action="store_true",
        help="generate and syntax-check all Pydantic peers without importing or timing",
    )
    args = parser.parse_args()

    all_cases = fixture_cases()
    cases = all_cases[: args.limit] if args.limit is not None else all_cases
    sample_cache = load_sample_cache(CHECKED_SAMPLE_CACHE)
    runtime_unsupported = load_string_manifest(RUNTIME_UNSUPPORTED_MANIFEST)
    unsatisfiable = load_unsatisfiable_manifest(UNSATISFIABLE_MANIFEST)
    validate_fixture_classifications(
        all_cases,
        sample_cache,
        runtime_unsupported,
        unsatisfiable,
    )
    output_root = args.output.resolve()
    models_root = output_root / "models" / "pydantic"
    results_path = (
        args.results.resolve()
        if args.results is not None
        else output_root / "results.json"
    )
    jsoncompat_version = package_version("jsoncompat")

    print(
        f"Python {platform.python_version()}, jsoncompat {jsoncompat_version}, "
        f"Pydantic {pydantic.__version__}, "
        f"datamodel-code-generator {package_version('datamodel-code-generator')}"
    )
    print(f"fixture schemas: {len(cases)}/{len(all_cases)}")

    generation_started = time.perf_counter()
    generation = generate_pydantic_models(
        cases,
        models_root,
        reuse_models=args.reuse_models,
    )
    generation_seconds = time.perf_counter() - generation_started
    generated_count = sum(
        outcome["status"] == "generated" for outcome in generation.values()
    )
    print(
        f"Pydantic generation: {generated_count}/{len(cases)} models in "
        f"{generation_seconds:.2f}s"
    )

    if args.generate_only:
        records = [
            {
                "case_id": case.case_id,
                "schema_digest": case.schema_digest,
                "source_path": str(case.source_path.relative_to(REPO_ROOT)),
                "schema_index": case.schema_index,
                "jsoncompat_status": (
                    "generated"
                    if case.jsoncompat_model_path is not None
                    else "unsupported"
                ),
                "pydantic_generation": generation[case.case_id],
            }
            for case in cases
        ]
        write_json(
            results_path,
            {
                "environment": {
                    "python": platform.python_version(),
                    "jsoncompat": jsoncompat_version,
                    "pydantic": pydantic.__version__,
                    "datamodel_code_generator": package_version(
                        "datamodel-code-generator"
                    ),
                },
                "fixture_cases_total": len(all_cases),
                "fixture_cases_selected": len(cases),
                "generation_seconds": generation_seconds,
                "configuration": {
                    "profile": args.profile,
                    "generator_configuration": GENERATOR_CONFIGURATION,
                },
                "records": records,
            },
        )
        print(f"generation manifest: {results_path}")
        return

    records: list[dict[str, Any]] = []

    for position, case in enumerate(cases, start=1):
        generation_outcome = generation[case.case_id]
        record: dict[str, Any] = {
            "case_id": case.case_id,
            "schema_digest": case.schema_digest,
            "schema_bytes": len(case.schema_json.encode()),
            "source_path": str(case.source_path.relative_to(REPO_ROOT)),
            "schema_index": case.schema_index,
            "jsoncompat_model_path": (
                str(case.jsoncompat_model_path.relative_to(REPO_ROOT))
                if case.jsoncompat_model_path is not None
                else None
            ),
            "jsoncompat_error_path": (
                str(case.jsoncompat_error_path.relative_to(REPO_ROOT))
                if case.jsoncompat_error_path is not None
                else None
            ),
            "pydantic_generation": generation_outcome,
        }

        # The jsoncompat end-to-end corpus is independent of whether Pydantic can
        # generate, import, or exactly represent this schema. Pydantic ratios are
        # added later only after the semantic-equivalence screen succeeds.
        if case.jsoncompat_model_path is None:
            record["jsoncompat_end_to_end_status"] = "codegen_unsupported"
        else:
            jsoncompat_e2e_module_name: str | None = None
            expected_runtime_error = runtime_unsupported.get(case.case_id)
            unsatisfiable_fixture = unsatisfiable.get(case.case_id)
            try:
                jsoncompat_e2e_module_name, jsoncompat_e2e_module = import_module(
                    case.jsoncompat_model_path,
                    case.case_id,
                    "jsoncompat_e2e",
                )
                jsoncompat_e2e_model = model_from_module(
                    jsoncompat_e2e_module,
                    "JSONCOMPAT_MODEL",
                )
                if expected_runtime_error is not None:
                    if not case.fixture_candidates:
                        raise AssertionError(
                            "runtime-unsupported fixture has no value with which to "
                            "verify its checked conversion error"
                        )
                    jsoncompat_e2e_model.deserialize(
                        canonical_json(case.fixture_candidates[0])
                    )
                    raise AssertionError(
                        "runtime-unsupported manifest is stale: checked conversion "
                        "succeeded"
                    )
                elif unsatisfiable_fixture is not None:
                    record["jsoncompat_end_to_end_status"] = "unsatisfiable"
                    record["jsoncompat_end_to_end_error"] = unsatisfiable_fixture[
                        "reason"
                    ]
                else:
                    jsoncompat_prepared, jsoncompat_preparation_error = (
                        prepare_jsoncompat_value(
                            case,
                            jsoncompat_e2e_model,
                            sample_cache,
                        )
                    )
                    if jsoncompat_prepared is None:
                        raise RuntimeError(
                            f"checked-in sample did not produce a stable round trip: "
                            f"{jsoncompat_preparation_error}"
                        )
                    value_bytes = len(jsoncompat_prepared.wire.encode())
                    record.update(
                        {
                            "jsoncompat_end_to_end_status": "benchmarked",
                            "jsoncompat_end_to_end_sample_source": jsoncompat_prepared.source,
                            "jsoncompat_end_to_end_value": jsoncompat_prepared.value,
                            "jsoncompat_end_to_end_value_bytes": value_bytes,
                            "jsoncompat_end_to_end_size_bucket": size_bucket(
                                value_bytes
                            ),
                            "jsoncompat_end_to_end_timings_ns": benchmark_jsoncompat_end_to_end(
                                jsoncompat_e2e_model,
                                jsoncompat_prepared,
                                iterations=args.iterations,
                                repeats=args.repeats,
                            ),
                        }
                    )
            except Exception as error:  # noqa: BLE001 - exact failures are classified
                actual_error = f"{type(error).__name__}: {error}"
                if expected_runtime_error is None:
                    raise RuntimeError(
                        f"unexpected jsoncompat end-to-end failure for {case.case_id}: "
                        f"{actual_error}"
                    ) from error
                if actual_error == expected_runtime_error:
                    record["jsoncompat_end_to_end_status"] = "runtime_unsupported"
                else:
                    raise RuntimeError(
                        f"runtime-unsupported manifest drift for {case.case_id}: "
                        f"expected {expected_runtime_error!r}, got {actual_error!r}"
                    ) from error
                record["jsoncompat_end_to_end_error"] = actual_error
            finally:
                if jsoncompat_e2e_module_name is not None:
                    sys.modules.pop(jsoncompat_e2e_module_name, None)

        if expected_error := runtime_unsupported.get(case.case_id):
            record["pydantic_import"] = {"status": "not_attempted"}
            record["status"] = cast(Status, "jsoncompat_validation_unsupported")
            record["error"] = expected_error
            records.append(record)
            continue

        if unsatisfiable_fixture := unsatisfiable.get(case.case_id):
            record["pydantic_import"] = {"status": "not_attempted"}
            record["status"] = cast(Status, "unsatisfiable")
            record["error"] = unsatisfiable_fixture["reason"]
            records.append(record)
            continue

        if generation_outcome["status"] != "generated":
            record["pydantic_import"] = {"status": "not_generated"}
            record["status"] = cast(
                Status,
                (
                    "jsoncompat_unsupported"
                    if case.jsoncompat_model_path is None
                    else "pydantic_codegen_error"
                ),
            )
            records.append(record)
            continue

        pydantic_path = REPO_ROOT / generation_outcome["path"]
        pydantic_module_name: str | None = None
        jsoncompat_module_name: str | None = None
        try:
            try:
                pydantic_module_name, pydantic_module = import_module(
                    pydantic_path, case.case_id, "pydantic"
                )
                pydantic_type = model_from_module(pydantic_module, "PYDANTIC_MODEL")
                pydantic_adapter: TypeAdapter[Any] = TypeAdapter(pydantic_type)
                unsafe_recursive_root = unproductive_recursive_root(pydantic_module)
                record["pydantic_import"] = {"status": "imported"}
            except Exception as error:  # noqa: BLE001 - record generated import failures
                message = f"{type(error).__name__}: {error}"
                record["pydantic_import"] = {
                    "status": "error",
                    "error": message,
                }
                record["status"] = cast(
                    Status,
                    (
                        "jsoncompat_unsupported"
                        if case.jsoncompat_model_path is None
                        else "pydantic_import_error"
                    ),
                )
                record["error"] = message
                records.append(record)
                continue

            if case.jsoncompat_model_path is None:
                record["status"] = cast(Status, "jsoncompat_unsupported")
                records.append(record)
                continue

            try:
                jsoncompat_module_name, jsoncompat_module = import_module(
                    case.jsoncompat_model_path, case.case_id, "jsoncompat"
                )
                jsoncompat_model = model_from_module(
                    jsoncompat_module, "JSONCOMPAT_MODEL"
                )
            except Exception as error:  # noqa: BLE001 - record generated import failures
                record["status"] = cast(Status, "jsoncompat_import_error")
                record["error"] = f"{type(error).__name__}: {error}"
                records.append(record)
                continue

            if unsafe_recursive_root is not None:
                record["status"] = cast(Status, "pydantic_semantic_mismatch")
                record["semantic_probes"] = 0
                record["semantic_mismatches"] = 1
                record["semantic_mismatch_examples"] = [
                    {
                        "reason": (
                            "datamodel-code-generator emitted an unproductive "
                            f"recursive RootModel: {unsafe_recursive_root}"
                        )
                    }
                ]
                records.append(record)
                continue

            try:
                probe_count, mismatch_count, mismatch_examples = semantic_mismatches(
                    case,
                    pydantic_adapter,
                )
            except Exception as error:  # noqa: BLE001 - oracle failures are explicit
                record["status"] = cast(Status, "jsoncompat_validation_unsupported")
                record["error"] = (
                    f"semantic conformance oracle failed: {type(error).__name__}: {error}"
                )
                records.append(record)
                continue
            record["semantic_probes"] = probe_count
            record["semantic_mismatches"] = mismatch_count
            if mismatch_count:
                record["status"] = cast(Status, "pydantic_semantic_mismatch")
                record["semantic_mismatch_examples"] = mismatch_examples
                records.append(record)
                continue

            prepared, preparation_error = prepare_value(
                case,
                jsoncompat_model,
                pydantic_adapter,
                sample_cache,
            )
            if prepared is None:
                record["status"] = cast(Status, "no_shared_value")
                record["error"] = preparation_error
                records.append(record)
                continue

            value_bytes = len(prepared.wire.encode())
            record.update(
                {
                    "status": cast(Status, "benchmarked"),
                    "sample_source": prepared.source,
                    "value": prepared.value,
                    "value_bytes": value_bytes,
                    "size_bucket": size_bucket(value_bytes),
                    "pydantic_python_compatible": prepared.pydantic_python_compatible,
                    "timings_ns": benchmark_case(
                        jsoncompat_model,
                        pydantic_adapter,
                        prepared,
                        iterations=args.iterations,
                        repeats=args.repeats,
                    ),
                }
            )
            records.append(record)
        finally:
            if pydantic_module_name is not None:
                sys.modules.pop(pydantic_module_name, None)
            if jsoncompat_module_name is not None:
                sys.modules.pop(jsoncompat_module_name, None)

        if position % 25 == 0 or position == len(cases):
            benchmarked = sum(record["status"] == "benchmarked" for record in records)
            print(
                f"prepared and benchmarked: {position}/{len(cases)} "
                f"({benchmarked} paired)",
                flush=True,
            )

    expected_end_to_end = sum(
        case.jsoncompat_model_path is not None
        and case.case_id not in runtime_unsupported
        and case.case_id not in unsatisfiable
        for case in cases
    )
    actual_end_to_end = sum(
        record.get("jsoncompat_end_to_end_status") == "benchmarked"
        for record in records
    )
    if actual_end_to_end != expected_end_to_end:
        raise RuntimeError(
            "jsoncompat end-to-end benchmark coverage is incomplete: "
            f"expected {expected_end_to_end}, got {actual_end_to_end}"
        )
    summaries = [
        summarize_comparison(records, comparison) for comparison in COMPARISONS
    ]
    jsoncompat_end_to_end_summaries = [
        summarize_jsoncompat_operation(records, name, timing_key)
        for name, timing_key in JSONCOMPAT_END_TO_END_OPERATIONS
    ]
    result = {
        "environment": {
            "python": platform.python_version(),
            "platform": platform.platform(),
            "jsoncompat": jsoncompat_version,
            "pydantic": pydantic.__version__,
            "datamodel_code_generator": package_version("datamodel-code-generator"),
        },
        "configuration": {
            "profile": args.profile,
            "iterations": args.iterations,
            "repeats": args.repeats,
            "generator_configuration": GENERATOR_CONFIGURATION,
        },
        "fixture_cases_total": len(all_cases),
        "fixture_cases_selected": len(cases),
        "generation_seconds": generation_seconds,
        "jsoncompat_end_to_end_summaries": jsoncompat_end_to_end_summaries,
        "summaries": summaries,
        "records": records,
    }
    write_json(results_path, result)
    print_summary(records, summaries, jsoncompat_end_to_end_summaries)
    print(f"\nDetailed results: {results_path}")
    print(f"Generated Pydantic models: {models_root}")


if __name__ == "__main__":
    main()
