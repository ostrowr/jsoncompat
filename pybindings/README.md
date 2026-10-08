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

At import, the runtime creates Python classes and compact field descriptions,
loads distinct precomputed programs once per module, checks their format, and
binds actual class/slot addresses. Repeated validation checks and scalar guards
are shared by the generated programs; unaliased fields reuse one name index.
Generated-model import and first use do not resolve type annotations or compile schemas,
regexes, or dataclass methods. Interpreter startup and native-library loading
remain normal process startup costs.

Generated classes retain frozen/slotted dataclass behavior, field metadata,
constructor signatures, equality, hashing, repr, pickle support, and
`dataclasses.replace`. Common frozen-dataclass methods are shared across classes
to reduce generated code and per-model memory. Full `dataclasses.Field` objects
and their metadata are created and cached only when explicitly requested by
reflection or utilities such as `dataclasses.fields`, `asdict`, or `replace`.
Construction, repr, equality, hashing, and JSON I/O do not trigger that work.
The private `__dataclass_fields__` attribute is a mapping rather than a plain
dict. JSON/YAML/MessagePack APIs and writer/reader restrictions are unchanged. Checked serialization validates current
state, including models constructed with `skip_validation=True` or subsequently
modified through `object.__setattr__`.

Where conversion proves the schema constraints, parsing and serialization check
scalar constraints directly and avoid a complete intermediate JSON tree or a
second schema walk. Other schemas use the same precompiled general validator;
ambiguous unions retain schema-based selection. Ordinary self-referential and
mutually recursive models using local `$ref` are supported, with a runtime depth
guard. The specialized `$dynamicRef` and `$recursiveRef` keywords, nonlocal
reference resolution, custom vocabularies, and regex backreferences/atomic
groups/subroutine calls currently fail during generation. Unicode word boundaries
and lookarounds use prebuilt operations. Complex regex compositions have an
execution budget; exhausting it rejects validation, including inside negation,
conditionals, or property patterns. Unsupported features fail explicitly;
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
release extension (2026-10-08). Every round-trip case uses nine rotating-order
samples and the split public/private artifacts:

| Workload | JSON bytes | Generated (µs) | Pydantic (µs) | Less elapsed time |
| --- | ---: | ---: | ---: | ---: |
| Small nested model | 228 | 2.503 | 4.068 | 38.5% |
| 1,365-node recursive tree | 268,847 | 2,290 | 3,803 | 39.8% |
| 21,845-node recursive tree | 4,350,138 | 37,093 | 64,904 | 42.8% |
| 1,000-field model | 65,391 | 237 | 398 | 40.4% |
| 201 generated classes | 159,352 | 603 | 1,122 | 46.2% |
| 12,000 records with Unicode strings | 9,622,824 | 44,701 | 74,811 | 40.2% |

Both benchmark suites pass `--assert-target`, including the deeper recursive
tree. All six comparison cases take at least 30% less elapsed time than Pydantic.
The native CPython path probes a bounded prefix of long strings. When it
encounters non-ASCII text there, it allocates the final character width and
length once, avoiding repeated widening and resizing. Other strings and Python
runtimes retain the standard constructors. Validation remains enabled.

In a separate alternating before/after comparison, the 9.6 MB round trip fell
from 46.61 ms to 40.55 ms during this optimization pass. Three-sample controls
using the same records with different string contents put the final ASCII-only
round trip within 1% of the previous runtime (14.18 versus 14.29 ms); Latin-1
and BMP controls take 8.9% and 18.0% less time, respectively.

With runtimes preloaded, the 201-class schema imports in
4.71 ms versus 109.79 ms for Pydantic. Its one-step generation and Python
bytecode build takes 217 ms. The small model imports in 0.42 ms versus
1.09 ms; its first round trip takes 68 µs versus 63 µs. Pydantic's common model
machinery is warmed before timing model import, just as the jsoncompat runtime
is preloaded. Full fresh-process import plus first round trip is
42.1 ms versus 103.7 ms. First-call measurements include ordinary allocation and cache
warming, but no deferred schema or model compilation.

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

Remeasured after this optimization pass, with all model identities kept alive
and garbage collection enabled:

| Fields/class | Implementation | Classes retained | Import seconds | Peak GiB |
| ---: | --- | ---: | ---: | ---: |
| 5 | Generated | 200,000 | 3.80 | 1.13 |
| 5 | Pydantic | 200,000 | 61.51 | 4.15 |
| 20 | Generated | 161,800 | 6.40 | 2.00 |
| 20 | Pydantic | 31,100 | 27.21 | 2.01 |
| 200 | Generated | 18,800 | 4.98 | 2.01 |
| 200 | Pydantic | 3,600 | 27.00 | 2.05 |

The 5-field case actually reaches 200,000 classes: generated models take
93.8% less import time than Pydantic. The previous generated implementation took
7.00 seconds and 2.67 GiB; this pass reduces that to 3.80 seconds and
1.13 GiB. The 20- and 200-field cases stop at the 2 GiB budget; those are
measured partial runs, not 200,000-class projections. Cached private companion
constants are shared across copies of each shard. Within a shard, identical
validation programs also share native storage. These memory figures do not
predict the size of arbitrary unique schemas.

A separate import profile alternates the preserved release `abd47c27` from
before this pass with the new implementation (nine fresh-process samples per version,
precompiled bytecode, runtime preloaded). Each shard contains 100 classes:

| Fields/class | Before: 100-class import | After: 100-class import | Less elapsed time | Current install/class |
| ---: | ---: | ---: | ---: | ---: |
| 5 | 2.31 ms | 1.51 ms | 34.7% | 1.20 µs |
| 20 | 6.12 ms | 2.76 ms | 54.9% | 1.35 µs |
| 200 | 49.43 ms | 18.98 ms | 61.6% | 2.24 µs |

`install_model` attaches constructor signatures and compressed schema sources.
Class creation and native binding happen separately. Compact field descriptions
avoid constructing full reflection metadata; the native binder reuses field
names and identical validation programs. Generation interns repeated scalar
checks and guards. Even the 1,000-field workload needs only two distinct scalar
guards, while every field still enforces its own constraints.

To verify that improvements also apply when native programs cannot be shared,
`--distinct-constraints` gives each class in a shard different numeric/string
bounds. The same alternating import profile gives:

| Fields/class | Before: distinct constraints | After: distinct constraints | Less elapsed time |
| ---: | ---: | ---: | ---: |
| 5 | 2.38 ms | 1.85 ms | 22.0% |
| 20 | 6.14 ms | 3.38 ms | 45.0% |
| 200 | 50.68 ms | 21.65 ms | 57.3% |

With distinct constraints in each shard, the retained-class experiment imports
200,000 five-field classes in 4.23 seconds and 1.76 GiB. Under the 2 GiB budget,
it retains 113,900 twenty-field classes or 15,500 two-hundred-field classes.
The cached-file/shared-constant caveat still applies.

Reproduce the distinct-constraint build and retained-class run with a separate
artifact directory (the release extension and CLI must already be built):

```bash
PYTHONPATH=pybindings JSONCOMPAT_NATIVE_PROFILE=release uv run --project pybindings python pybindings/bench_dataclasses_imports.py build --fields 20 --distinct-constraints --directory target/python-codegen/imports-distinct
PYTHONPATH=pybindings JSONCOMPAT_NATIVE_PROFILE=release uv run --project pybindings python pybindings/bench_dataclasses_imports.py generated --fields 20 --distinct-constraints --directory target/python-codegen/imports-distinct
```

`Model.__jsoncompat_schema__` decompresses and caches the original source only
on explicit access; imports, first round trips, and validation errors do not
expand it. Dataclass reflection similarly materializes full field metadata only
when requested. Generation removes compilation from startup; creating and
connecting runtime objects still costs time and memory.

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
