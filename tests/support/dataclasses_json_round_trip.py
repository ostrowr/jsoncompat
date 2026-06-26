"""Batched JSON round-trip driver for every generated dataclass fixture."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
from dataclasses import is_dataclass
from pathlib import Path
from types import ModuleType
from typing import Any, TypedDict, cast


type JsonScalar = None | bool | int | float | str
type JsonValue = JsonScalar | list["JsonValue"] | dict[str, "JsonValue"]


class FixtureCase(TypedDict):
    case_id: str
    module_path: str
    source_path: str
    schema_index: int | None
    expected_schema_digest: str | None
    candidates: list[JsonValue]
    runtime_unsupported: str | None
    unsatisfiable: str | None


class FixtureBatch(TypedDict):
    cases: list[FixtureCase]


def load_module(case_id: str, module_path: Path, index: int) -> ModuleType:
    module_name = f"_jsoncompat_fixture_round_trip_{index:04}"
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"{case_id}: could not create an import spec")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        sys.modules.pop(module_name, None)
        raise
    return module


def json_equivalent(left: JsonValue, right: JsonValue) -> bool:
    if isinstance(left, dict) and isinstance(right, dict):
        return left.keys() == right.keys() and all(
            json_equivalent(left[key], right[key]) for key in left
        )
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(
            json_equivalent(left_item, right_item)
            for left_item, right_item in zip(left, right, strict=True)
        )
    if isinstance(left, bool) or isinstance(right, bool):
        return isinstance(left, bool) and isinstance(right, bool) and left == right
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return left == right
    return type(left) is type(right) and left == right


def canonical_json(value: JsonValue) -> str:
    return json.dumps(
        value,
        allow_nan=False,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    )


def fixture_schema(case: FixtureCase) -> JsonValue:
    root = cast(JsonValue, json.loads(Path(case["source_path"]).read_text()))
    schema_index = case["schema_index"]
    if schema_index is None:
        return root
    if not isinstance(root, list):
        raise AssertionError(f"{case['case_id']}: indexed fixture is not an array")
    item = root[schema_index]
    if not isinstance(item, dict) or "schema" not in item:
        raise AssertionError(f"{case['case_id']}: fixture item has no schema")
    return item["schema"]


def assert_round_trip(
    *,
    case_id: str,
    candidate_index: int,
    candidate: JsonValue,
    model_type: Any,
    trusted: bool,
) -> None:
    wire = canonical_json(candidate)
    try:
        model = model_type.deserialize(wire, skip_validation=trusted)
        if not isinstance(model, model_type) or not is_dataclass(model):
            raise AssertionError(
                f"{case_id} candidate #{candidate_index}: deserialize returned "
                f"{type(model).__name__}, expected a {model_type.__name__} dataclass"
            )
        emitted_wire = cast(Any, model).serialize(skip_validation=trusted)
        emitted = cast(JsonValue, json.loads(emitted_wire))
    except BaseException as error:
        mode = "trusted" if trusted else "checked"
        raise AssertionError(
            f"{case_id} candidate #{candidate_index}: {mode} JSON round-trip "
            f"failed for {wire}: {type(error).__name__}: {error}"
        ) from error
    if not json_equivalent(candidate, emitted):
        mode = "trusted" if trusted else "checked"
        raise AssertionError(
            f"{case_id} candidate #{candidate_index}: {mode} JSON round-trip "
            f"changed the value: {wire} -> {canonical_json(emitted)}"
        )


def assert_runtime_unsupported(
    *,
    case_id: str,
    candidates: list[JsonValue],
    model_type: Any,
    expected_error: str,
) -> None:
    if not candidates:
        raise AssertionError(
            f"{case_id}: runtime-unsupported classification has no fixture value "
            "with which to verify the expected error"
        )
    wire = canonical_json(candidates[0])
    try:
        model_type.deserialize(wire)
    except BaseException as error:
        actual = f"{type(error).__name__}: {error}"
        if expected_error != actual:
            raise AssertionError(
                f"{case_id}: runtime-unsupported classification is stale; "
                f"expected {expected_error!r}, got {actual!r}"
            ) from error
    else:
        raise AssertionError(
            f"{case_id}: runtime-unsupported fixture now deserializes; remove its "
            "classification and exercise its complete fixture corpus"
        )


def main() -> None:
    batch = cast(FixtureBatch, json.load(sys.stdin))
    cases = batch["cases"]
    candidate_count = 0
    runtime_unsupported_count = 0
    unsatisfiable_count = 0
    failures: list[str] = []
    for case_index, case in enumerate(cases):
        case_id = case["case_id"]
        expected_runtime_error = case["runtime_unsupported"]
        unsatisfiable_reason = case["unsatisfiable"]
        expected_schema_digest = case["expected_schema_digest"]
        if expected_schema_digest is not None:
            actual_schema_digest = hashlib.sha256(
                canonical_json(fixture_schema(case)).encode()
            ).hexdigest()
            if actual_schema_digest != expected_schema_digest:
                failures.append(
                    f"{case_id}: checked-in classification/sample schema digest is "
                    f"stale; expected {actual_schema_digest}, got "
                    f"{expected_schema_digest}"
                )
                continue
        try:
            module = load_module(case_id, Path(case["module_path"]), case_index)
            model_type = module.JSONCOMPAT_MODEL
        except BaseException as error:
            actual = f"{type(error).__name__}: {error}"
            if expected_runtime_error == actual:
                runtime_unsupported_count += 1
            elif expected_runtime_error is None:
                failures.append(f"{case_id}: generated module import failed: {actual}")
            else:
                failures.append(
                    f"{case_id}: runtime-unsupported classification is stale; "
                    f"expected {expected_runtime_error!r}, got {actual!r}"
                )
            continue
        if expected_runtime_error is not None:
            runtime_unsupported_count += 1
            try:
                assert_runtime_unsupported(
                    case_id=case_id,
                    candidates=case["candidates"],
                    model_type=model_type,
                    expected_error=expected_runtime_error,
                )
            except AssertionError as error:
                failures.append(str(error))
            continue
        if unsatisfiable_reason is not None:
            if case["candidates"]:
                failures.append(
                    f"{case_id}: unsatisfiable fixture declares valid candidates"
                )
            else:
                unsatisfiable_count += 1
            continue
        for candidate_index, candidate in enumerate(case["candidates"]):
            candidate_count += 1
            try:
                assert_round_trip(
                    case_id=case_id,
                    candidate_index=candidate_index,
                    candidate=candidate,
                    model_type=model_type,
                    trusted=False,
                )
                assert_round_trip(
                    case_id=case_id,
                    candidate_index=candidate_index,
                    candidate=candidate,
                    model_type=model_type,
                    trusted=True,
                )
            except AssertionError as error:
                failures.append(str(error))

    if failures:
        raise AssertionError(
            f"{len(failures)} generated-dataclass fixture round-trip failures:\n"
            + "\n".join(f"- {failure}" for failure in failures)
        )

    print(
        canonical_json(
            {
                "candidates": candidate_count,
                "checked_round_trips": candidate_count,
                "generated_cases": len(cases),
                "runtime_unsupported": runtime_unsupported_count,
                "trusted_round_trips": candidate_count,
                "unsatisfiable": unsatisfiable_count,
            }
        )
    )


if __name__ == "__main__":
    main()
