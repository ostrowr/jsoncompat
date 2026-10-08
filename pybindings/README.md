# jsoncompat

Python bindings for checking compatibility of evolving JSON Schemas and generating example values.

## Installation

Install from PyPI:

```bash
pip install jsoncompat==0.4.3
```

Releases also provide a source distribution for platforms without a matching
wheel. To request a source build explicitly:

```bash
pip install --no-binary=jsoncompat jsoncompat
```

Source builds require Python 3.12 or newer, a current stable Rust toolchain,
and the platform's native linker/build tools. Network access is required to
fetch the build backend, Rust crates, and the pinned Git validator dependency.
The archive includes the local Rust workspace dependencies and Python sources;
you do not need a repository checkout.

## Quick start

```python
import jsoncompat as jsc

old_schema = '{"type": "string"}'
new_schema = '{"type": "number"}'

is_compatible = jsc.check_compat(old_schema, new_schema, jsc.Role.BOTH)
print(is_compatible)

# Generate example values for a schema
generator = jsc.generator_for(old_schema)
example = generator.generate_value(depth=5)
print(example)
```

## API

- `check_compat(old_schema_json: str, new_schema_json: str, role: str = "both") -> bool`
  - `role` must be `"serializer"`, `"deserializer"`, or `"both"`.
  - Raises `ValueError` for invalid schemas or hard unsupported compatibility features such as non-integral `number.multipleOf`.
- `generator_for(schema_json: str) -> Generator`
  - Parses the schema once and returns a reusable generator.
  - `Generator.generate_value(depth: int = 5) -> str` returns a JSON string for one generated value accepted by the schema.
  - Raises `ValueError` when the schema is invalid, known to be unsatisfiable, or generation exhausts its retry budget.
- `validator_for(schema_json: str) -> Validator`
  - Parses the schema once and returns a reusable validator.
  - `Validator.is_valid_json(instance_json: str) -> bool` validates JSON strings against the parsed schema.
  - `Validator.is_valid_value(instance: JsonValue) -> bool` validates Python JSON-compatible values: `None`, `bool`, finite `int`/`float`, `str`, `list`, `tuple`, and `dict[str, ...]`.
- `generate_value(schema_json: str, depth: int = 5) -> str`
  - Deprecated. Use `generator_for(schema_json).generate_value(depth)` instead.
- `is_valid(schema_json: str, instance_json: str) -> bool`
  - Deprecated. Use `validator_for(schema_json).is_valid_json(instance_json)` instead.
- `jsoncompat.codegen.dataclasses` runtime helpers for generated dataclass models
- `Role.SERIALIZER`, `Role.DESERIALIZER`, and `Role.BOTH` are string constants accepted by `check_compat`.

Generated dataclasses use `from_value(...)` / `to_value(...)` for Python JSON
values and `deserialize(...)` / `serialize(...)` for encoded JSON, YAML, and
MessagePack. JSON is the default format. Install optional codecs with
`jsoncompat[yaml]` or `jsoncompat[msgpack]`. All direct constructors and
conversion methods accept keyword-only `skip_validation=True` when the caller
already guarantees schema validity. It skips only the attached JSON Schema
check; wire-format parsing and JSON-value normalization, runtime type
conversion, and reader/writer direction guards still apply.

Only classes emitted by `jsoncompat codegen --target dataclasses` implement
this runtime contract. The generator builds the shared conversion plan and
validation programs ahead of time. Import binds the plan to the generated
classes, and it is then shared across threads. Custom subclasses, construction
hooks, and Python defaults/default factories are outside the model-definition
surface.

Generated array and object fields accept ordinary lists and dictionaries at
construction boundaries, then store them as deeply immutable values exposed as
`Sequence[...]` and `Mapping[...]`. Use `to_value()` when a mutable JSON-value
tree is required by another API.

See the [canonical plain-schema example](../examples/dataclasses/demo.py) for an
ordinary generated model that both serializes and deserializes. The
[canonical stamped-schema example](../examples/stamp/demo.py) covers versioned
writer/reader envelopes and historical schemas.

## Generated model artifacts

Generation produces models ready to import. The Rust generator resolves model
references and types, compiles schema programs and regexes, and computes
conversion optimizations in the same command that emits Python:

```bash
jsoncompat codegen --target dataclasses schema.json --output models.py
```

The public `models.py` imports a private `_models_generated.py` companion at the
top, followed by the root model and readable field declarations. A binding call
after the classes connects them to the companion's constructor signatures,
schemas, and precomputed runtime programs. Imports support both package and
top-level modules.

Regeneration replaces the same two filenames. Generate them during your build
and deploy both files together; the pair is a single build artifact. Each file
is replaced atomically, but the two-file update is not a live-reload protocol.

Without `--output`, the same generator writes a self-contained module to stdout:

```bash
jsoncompat codegen --target dataclasses schema.json > models.py
```

Its public declarations appear first and implementation follows a marked divider.
These are two packaging layouts of one generated format, with identical behavior.
There is no unprepared model format, runtime fallback compiler, or separate Python
preparation command. Regenerate models produced by older jsoncompat versions.
The generator does not require a Python interpreter. Python's ordinary `.pyc`
cache is still interpreter-specific: wheel installation normally builds it;
source-only deployments should precompile it during packaging when cold import
latency matters.

At import, the runtime creates Python classes and dataclass metadata, loads the
precomputed programs, checks their format, and binds actual class/slot addresses.
Generated-model import and first use do not resolve type annotations or compile schemas,
regexes, or dataclass methods. Interpreter startup and native-library loading
remain normal process startup costs.

Generated classes retain frozen/slotted dataclass behavior, field metadata,
constructor signatures, equality, hashing, repr, pickle support, and
`dataclasses.replace`. Common frozen-dataclass methods are shared across classes
to reduce generated code and per-model memory. JSON/YAML/MessagePack APIs and
writer/reader restrictions are unchanged. Checked serialization validates current
state, including models constructed with `skip_validation=True` or subsequently
modified through `object.__setattr__`.

Where conversion proves the schema constraints, parsing and serialization check
scalar constraints directly and avoid a complete intermediate JSON tree or a
second schema walk. Other schemas use the same precompiled general validator;
ambiguous unions retain schema-based selection. Ordinary self-referential and
mutually recursive models using local `$ref` are supported, with a runtime depth
guard. The specialized `$dynamicRef` and `$recursiveRef` keywords, nonlocal
reference resolution, custom vocabularies, and regex backreferences/atomic
groups/subroutine calls currently fail during generation. Complex regex
compositions have an execution budget. Unsupported features fail explicitly;
constraints are never silently dropped or compiled later.

Benchmark the build cost, fully checked round trips, and fresh-process startup
against the same strict Pydantic peers used by the existing benchmarks:

```bash
just python-bench-codegen
just python-bench-codegen-large
```

The report at `target/python-codegen/benchmark.json` records all timing samples,
versions, payload sizes, build time, and startup boundaries. Execution order
rotates between implementations. `--assert-target` on
`pybindings/bench_dataclasses_codegen.py` requires generated-model elapsed time to be
at most 70% of Pydantic on **both** the small model and the recursive tree.
The large profile writes `target/python-codegen/large/benchmark.json` and enforces
the same target independently for a 1,000-field model, a schema generating 201
classes, and 12,000 records (about 9.6 MB of JSON, including non-ASCII strings).
It also measures model import and the first round trip separately with the
runtime already imported. Both implementations enforce the emitted length,
numeric, required-property, and additional-property constraints; invalid
values at the end of each large payload are checked before timing.
To increase recursive depth, run the standard script with `--depth 7 --fanout 4
--tree-iterations 10 --output target/python-codegen/deep/benchmark.json`.
Pydantic validates input; jsoncompat also validates output. These are measured
workloads, not a speed guarantee for every schema or machine. The test suite
checks generated models against the general schema validator
across every generated fixture, supplied examples, and deterministic mutations.

Measured on macOS 26.6.2 arm64 with Python 3.12.2, Pydantic 2.13.4, and the
release extension (2026-10-08; median of nine rotating-order samples). These
measurements use the split public/private artifacts:

| Workload | JSON bytes | Generated (µs) | Pydantic (µs) | Less elapsed time |
| --- | ---: | ---: | ---: | ---: |
| Small nested model | 228 | 2.449 | 3.915 | 37.4% |
| 1,365-node recursive tree | 268,847 | 2,114 | 3,366 | 37.2% |
| 21,845-node recursive tree | 4,350,138 | 36,250 | 62,012 | 41.5% |
| 1,000-field model | 65,391 | 264 | 383 | 31.1% |
| 201 generated classes | 159,352 | 655 | 1,058 | 38.1% |
| 12,000 records with Unicode strings | 9,622,824 | 47,913 | 69,473 | 31.0% |

With runtimes preloaded, the 201-class schema imports in 10.9 ms versus
108.5 ms for Pydantic. Its one-step generation and Python bytecode build takes
215 ms. The small model imports in 0.45 ms versus 0.90 ms for Pydantic; the first
round trip takes 46 µs versus 47 µs. Pydantic's common model machinery is warmed
before timing model import, just as the jsoncompat runtime is preloaded. Full
fresh-process import plus first round trip is 33.7 ms versus 85.7 ms.

To measure independent-class imports with a retained-memory budget:

```bash
just python-bench-imports 5 6000
just python-bench-imports 20 2048
just python-bench-imports 200 2048
```

This benchmark builds 100-class shards, then imports distinct retained model
classes until it reaches 200,000 classes or the memory/time budget. Bytecode is
built beforehand. It reuses cached shard files under distinct module names,
so it measures class setup and retained memory, excluding unique-file I/O and
a real package's dependency graph. Results and build costs are written to
`target/python-codegen/imports/`. Wider models may reach the default 2 GiB
budget before 200,000 classes; reports mark that boundary explicitly.

A single bounded run on the same machine produced the following results. All
model identities remain alive, and garbage collection stays enabled:

| Fields/class | Implementation | Classes retained | Import seconds | Peak GiB |
| ---: | --- | ---: | ---: | ---: |
| 5 | Generated | 200,000 | 8.36 | 2.79 |
| 5 | Pydantic | 200,000 | 58.46 | 4.19 |
| 20 | Generated | 46,100 | 4.33 | 2.00 |
| 20 | Pydantic | 31,000 | 26.13 | 2.00 |
| 200 | Generated | 5,000 | 3.57 | 2.01 |
| 200 | Pydantic | 3,600 | 26.17 | 2.05 |

The 5-field case actually reaches 200,000 classes: generated models use 85.7%
less import time. The 20- and 200-field cases stop at the 2 GiB budget;
those rows are measured partial runs, not 200,000-class projections. Cached
private companion constants are shared across copies of each generated shard,
so these memory figures do not predict the size of arbitrary unique schemas.

Import still performs per-class setup. Profiling `install_model` with 100-class
shards, precompiled Python bytecode, and the runtime preloaded (nine fresh-process
samples, same environment) gives these median costs:

| Fields/class | `install_model` per class | Whole 100-class import |
| ---: | ---: | ---: |
| 5 | 8.4 µs | 3.47 ms |
| 20 | 9.5 µs | 7.23 ms |
| 200 | 20.1 µs | 53.75 ms |

`install_model` attaches constructor annotations, dataclass metadata, and shared
methods. Class creation and field metadata construction happen before it;
loading native validation programs and binding class/slot references happen
separately. All three contribute to import time. A linear extrapolation of
`install_model` alone is about 1.7–4 seconds for 200,000 classes; that is not a
measurement of total import time for the wider schemas. Generation removes
compilation from startup, but creating and connecting runtime objects still
costs time and memory.

Schemas are passed as JSON strings. `check_compat` returns a boolean verdict and raises `ValueError` for invalid JSON, invalid schemas, or hard unsupported compatibility cases.

## More detail

- [Basic demo](https://github.com/ostrowr/jsoncompat/blob/main/examples/python/basic/demo.py)
- https://jsoncompat.com
- [Repository README](https://github.com/ostrowr/jsoncompat/blob/main/readme.md)
- [Developer guide](https://github.com/ostrowr/jsoncompat/blob/main/developing.md)

## Benchmarks

Run the generated dataclass runtime microbenchmark from the repository root:

```bash
just python-bench
```

The benchmark pins Pydantic v2 and compares the same valid nested payload using
strict, frozen Pydantic models. Pydantic's dump methods do not revalidate the
model, so their closest jsoncompat comparison is the `skip_validation=True`
serialization path.

To measure fresh-interpreter startup separately from steady-state conversion,
including generated-module import, first trusted use, first checked use, and an
equivalent Pydantic v2 graph:

```bash
just python-bench-startup
```

Each sample runs in a new Python process and reports both absolute time and the
increment over an empty process. Results are written under
`target/python-benchmark` alongside the other repeatable benchmark profiles.

For a larger workload, benchmark a balanced recursive graph containing
discriminated unions, lists, mappings, and omitted fields:

```bash
just python-bench-scale
```

The default depth and fanout produce 1,365 model nodes. Override them, along
with the iteration and repeat counts, as positional recipe arguments. The
underlying script also accepts `--profile` to print cumulative Python profiles
for checked and trusted JSON deserialization.

To benchmark JSON -> generated dataclass -> JSON across the complete checked-in
JSON Schema fixture corpus, and compare Pydantic v2 wherever its generated peer
passes the semantic-equivalence screen:

```bash
just python-bench-fixtures
```

This includes every embedded fuzz schema and both sides of every backcompat
fixture. Jsoncompat timings cover every fixture with a representable schema and
valid sample independently of Pydantic coverage. Generated Pydantic modules
and the detailed JSON report are written to
`target/python-fixture-benchmark`. The report retains explicit entries for
jsoncompat or Pydantic generation failures, semantic mismatches, and schemas
without a shared valid value; those cases are never silently removed from the
denominator. Before timing, generated Pydantic validators are screened against
the jsoncompat schema validator with every fixture test plus deterministic type
and property mutations. This prevents ignored JSON Schema keywords from looking
like performance wins. Pydantic uses one precompiled `TypeAdapter` per generated
type, as recommended for repeated validation. Shared generated values are
checked in separately from the generated model artifacts so fresh clones
benchmark the same values.

## License

MIT License. See:

- https://github.com/ostrowr/jsoncompat/blob/main/LICENSE
