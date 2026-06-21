"""Benchmark a large recursive generated-dataclass-style object graph.

The workload is a balanced directory tree whose leaves contain lists, mappings,
optional fields, and discriminated recursive unions. An equivalent strict,
frozen Pydantic v2 graph provides a useful point of comparison.
"""

from __future__ import annotations

import argparse
import cProfile
import gc
import json
import platform
import pstats
import statistics
import time
from dataclasses import dataclass
from typing import Annotated, Any, Callable, ClassVar, Literal

import pydantic
from pydantic import BaseModel, ConfigDict, Field, RootModel

from jsoncompat.codegen import dataclasses as dc


type JsonObject = dict[str, Any]


NODE_DEFINITIONS: JsonObject = {
    "node": {
        "oneOf": [
            {"$ref": "#/$defs/file"},
            {"$ref": "#/$defs/directory"},
        ]
    },
    "file": {
        "type": "object",
        "required": ["kind", "name", "size", "labels", "metadata"],
        "properties": {
            "kind": {"const": "file"},
            "name": {"type": "string", "minLength": 1},
            "size": {"type": "integer", "minimum": 0},
            "checksum": {"type": ["string", "null"]},
            "labels": {"type": "array", "items": {"type": "string"}},
            "metadata": {
                "type": "object",
                "additionalProperties": {"type": "string"},
            },
        },
        "additionalProperties": False,
    },
    "directory": {
        "type": "object",
        "required": ["kind", "name", "owner", "children", "metadata"],
        "properties": {
            "kind": {"const": "directory"},
            "name": {"type": "string", "minLength": 1},
            "owner": {"type": ["string", "null"]},
            "children": {
                "type": "array",
                "items": {"$ref": "#/$defs/node"},
            },
            "metadata": {
                "type": "object",
                "additionalProperties": {"type": "string"},
            },
        },
        "additionalProperties": False,
    },
}


def schema_for(definition: str) -> str:
    return json.dumps(
        {
            "$defs": NODE_DEFINITIONS,
            "$ref": f"#/$defs/{definition}",
        },
        separators=(",", ":"),
        sort_keys=True,
    )


@dataclass(frozen=True, slots=True, kw_only=True)
class ScaleFile(dc.DataclassModel):
    __jsoncompat_schema__: ClassVar[str] = schema_for("file")

    checksum: dc.Omittable[str | None] = dc.field("checksum", omittable=True)
    kind: Literal["file"] = dc.field("kind")
    labels: list[str] = dc.field("labels")
    metadata: dict[str, str] = dc.field("metadata")
    name: str = dc.field("name")
    size: int = dc.field("size")


@dataclass(frozen=True, slots=True, kw_only=True)
class ScaleDirectory(dc.DataclassModel):
    __jsoncompat_schema__: ClassVar[str] = schema_for("directory")

    children: list[ScaleFile | ScaleDirectory] = dc.field("children")
    kind: Literal["directory"] = dc.field("kind")
    metadata: dict[str, str] = dc.field("metadata")
    name: str = dc.field("name")
    owner: str | None = dc.field("owner")


@dataclass(frozen=True, slots=True, kw_only=True)
class ScaleTree(dc.DataclassRootModel):
    __jsoncompat_schema__: ClassVar[str] = schema_for("node")

    root: ScaleFile | ScaleDirectory = dc.root_field()


class PydanticFile(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True, strict=True)

    checksum: str | None = None
    kind: Literal["file"]
    labels: list[str]
    metadata: dict[str, str]
    name: Annotated[str, Field(min_length=1)]
    size: Annotated[int, Field(ge=0)]


class PydanticDirectory(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True, strict=True)

    children: list[
        Annotated[
            PydanticFile | PydanticDirectory,
            Field(discriminator="kind"),
        ]
    ]
    kind: Literal["directory"]
    metadata: dict[str, str]
    name: Annotated[str, Field(min_length=1)]
    owner: str | None


class PydanticTree(
    RootModel[
        Annotated[
            PydanticFile | PydanticDirectory,
            Field(discriminator="kind"),
        ]
    ]
):
    model_config = ConfigDict(frozen=True, strict=True)


def build_payload(depth: int, fanout: int, index: int = 0) -> JsonObject:
    if depth == 0:
        value: JsonObject = {
            "kind": "file",
            "name": f"asset-{index}.bin",
            "size": index * 17 + 128,
            "labels": ["generated", f"shard-{index % 16}"],
            "metadata": {
                "contentType": "application/octet-stream",
                "storageClass": "standard",
            },
        }
        if index % 3 != 0:
            value["checksum"] = f"sha256:{index:064x}"
        return value

    return {
        "kind": "directory",
        "name": f"directory-{depth}-{index}",
        "owner": None if index % 5 == 0 else f"team-{index % 11}",
        "children": [
            build_payload(depth - 1, fanout, index * fanout + child_index + 1)
            for child_index in range(fanout)
        ],
        "metadata": {
            "region": f"region-{index % 4}",
            "tier": "hot" if depth < 3 else "archive",
        },
    }


def node_count(depth: int, fanout: int) -> int:
    if fanout == 1:
        return depth + 1
    return (fanout ** (depth + 1) - 1) // (fanout - 1)


def positive_int(raw_value: str) -> int:
    value = int(raw_value)
    if value < 1:
        raise argparse.ArgumentTypeError("value must be at least 1")
    return value


def nonnegative_int(raw_value: str) -> int:
    value = int(raw_value)
    if value < 0:
        raise argparse.ArgumentTypeError("value must be at least 0")
    return value


def benchmark(
    name: str,
    callback: Callable[[], Any],
    *,
    iterations: int,
    repeats: int,
) -> None:
    for _ in range(min(iterations, 10)):
        callback()

    samples: list[float] = []
    gc_was_enabled = gc.isenabled()
    gc.disable()
    try:
        for _ in range(repeats):
            start = time.perf_counter()
            for _ in range(iterations):
                callback()
            samples.append(time.perf_counter() - start)
    finally:
        if gc_was_enabled:
            gc.enable()

    median_ms = statistics.median(samples) / iterations * 1_000
    best_ms = min(samples) / iterations * 1_000
    print(f"{name:36} median={median_ms:9.3f}ms best={best_ms:9.3f}ms")


def profile(
    name: str,
    callback: Callable[[], Any],
    *,
    iterations: int,
) -> None:
    profiler = cProfile.Profile()

    def run() -> None:
        for _ in range(iterations):
            callback()

    profiler.runcall(run)
    print(f"\n{name} ({iterations} iterations, cumulative time)")
    pstats.Stats(profiler).strip_dirs().sort_stats("cumulative").print_stats(20)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--depth", type=nonnegative_int, default=5)
    parser.add_argument("--fanout", type=positive_int, default=4)
    parser.add_argument("--iterations", type=positive_int, default=100)
    parser.add_argument("--repeats", type=positive_int, default=5)
    parser.add_argument(
        "--profile",
        action="store_true",
        help="print cProfile results for checked and trusted JSON deserialization",
    )
    args = parser.parse_args()

    payload = build_payload(args.depth, args.fanout)
    payload_json = json.dumps(payload, separators=(",", ":"), sort_keys=True)
    instance = ScaleTree.from_value(payload)
    pydantic_instance = PydanticTree.model_validate(payload)
    wire = instance.serialize(skip_validation=True)

    assert instance.to_value() == payload
    assert pydantic_instance.model_dump(mode="json", exclude_unset=True) == payload
    assert json.loads(wire) == payload

    def run(name: str, callback: Callable[[], Any]) -> None:
        benchmark(
            name,
            callback,
            iterations=args.iterations,
            repeats=args.repeats,
        )

    print(f"Python {platform.python_version()}, Pydantic {pydantic.__version__}")
    print(
        f"recursive nodes={node_count(args.depth, args.fanout):,} "
        f"JSON bytes={len(payload_json.encode()):,} "
        f"depth={args.depth} fanout={args.fanout}"
    )

    run("jsoncompat from_value checked", lambda: ScaleTree.from_value(payload))
    run(
        "jsoncompat from_value trusted",
        lambda: ScaleTree.from_value(payload, skip_validation=True),
    )
    run("pydantic model_validate", lambda: PydanticTree.model_validate(payload))
    run("jsoncompat to_value checked", instance.to_value)
    run(
        "jsoncompat to_value trusted",
        lambda: instance.to_value(skip_validation=True),
    )
    run(
        "pydantic model_dump",
        lambda: pydantic_instance.model_dump(mode="json", exclude_unset=True),
    )
    run("jsoncompat serialize checked", instance.serialize)
    run(
        "jsoncompat serialize trusted",
        lambda: instance.serialize(skip_validation=True),
    )
    run(
        "pydantic model_dump_json",
        lambda: pydantic_instance.model_dump_json(exclude_unset=True),
    )
    run("jsoncompat deserialize checked", lambda: ScaleTree.deserialize(wire))
    run(
        "jsoncompat deserialize trusted",
        lambda: ScaleTree.deserialize(wire, skip_validation=True),
    )
    run(
        "pydantic model_validate_json",
        lambda: PydanticTree.model_validate_json(payload_json),
    )

    if args.profile:
        profile(
            "jsoncompat deserialize checked",
            lambda: ScaleTree.deserialize(wire),
            iterations=args.iterations,
        )
        profile(
            "jsoncompat deserialize trusted",
            lambda: ScaleTree.deserialize(wire, skip_validation=True),
            iterations=args.iterations,
        )


if __name__ == "__main__":
    main()
