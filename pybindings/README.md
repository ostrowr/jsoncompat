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
this runtime contract. Generated modules contain only dataclass declarations;
the first constructor or conversion call derives and caches one shared native
plan from their field metadata. Checked use then compiles the relevant JSON
Schema validator once, while `skip_validation=True` leaves it uncompiled. There
is no custom-subclass or Python-constructor fallback. Custom construction hooks
and Python defaults/default factories are therefore intentionally outside the
model-definition surface.

The conversion plan is shared across threads. Ordinary modules still initialize
it lazily; the optional model build below removes model-specific compilation
from both import and first use.

Generated array and object fields accept ordinary lists and dictionaries at
construction boundaries, then store them as deeply immutable values exposed as
`Sequence[...]` and `Mapping[...]`. Use `to_value()` when a mutable JSON-value
tree is required by another API.

See the [canonical plain-schema example](../examples/dataclasses/demo.py) for an
ordinary generated model that both serializes and deserializes. The
[canonical stamped-schema example](../examples/stamp/demo.py) covers versioned
writer/reader envelopes and historical schemas.

## Optional model build

Prepare generated modules in a packaging or deployment step:

```bash
jsoncompat codegen --target dataclasses schema.json > models.py
python -m jsoncompat.codegen.build models.py --output build/models.py
```

Applications import the prepared `build/models.py` as their model module. The
source `models.py` remains usable independently. Build input is trusted Python
code and is imported during preparation. Output must have a different path;
the source is never overwritten, and the destination source is replaced
atomically only after preparation succeeds. The build also writes ordinary
Python bytecode for its interpreter. When packaging for another Python version,
let wheel installation or `python -m compileall build` regenerate that cache.

Preparation performs annotation resolution, conversion-graph analysis,
dataclass method generation, schema validation/compilation, and regex
compilation. It stores portable validation instructions, precompiled automata
in both byte orders, and conversion optimizations in a self-contained Python
module. At import, the runtime checks the artifact format and graph references,
loads programs, and binds the actual Python classes and slot offsets. First use
does not compile or inspect anything. Python interpreter startup, native-library
loading, and allocation of classes/runtime objects still happen in the process.

Prepared classes retain frozen/slotted dataclass behavior, field metadata,
constructor signatures, equality, hashing, repr, pickle support, and
`dataclasses.replace`. Their JSON/YAML/MessagePack APIs and writer/reader
restrictions are unchanged. Checked serialization validates current state,
including models constructed with `skip_validation=True` or subsequently
modified through `object.__setattr__`.

For schemas whose conversion checks imply validation, scalar constraints are
checked during parsing and output; the runtime can construct model fields
directly without allocating a complete intermediate JSON tree or doing a
second schema walk. Field keys are escaped during the build. Other schemas use
the prepared validator, including combined applicator
annotations for `unevaluatedProperties` and `unevaluatedItems`. Ambiguous unions
retain schema-based selection. The builder rejects features it cannot prepare
rather than deferring compilation or dropping constraints. In particular,
dynamic/recursive references, nonlocal reference resolution, custom
vocabularies, and regex backreferences/atomic groups/subroutine calls are not
supported by this build format. Complex regex compositions have an execution
budget, and recursive validation has a depth guard. Keep using the original
generated module when preparation reports an unsupported schema. Rebuild
artifacts when upgrading jsoncompat; incompatible formats fail at import.

Benchmark the build cost, fully checked round trips, and fresh-process startup
against the same strict Pydantic peers used by the existing benchmarks:

```bash
just python-bench-prepared
just python-bench-prepared-large
```

The report at `target/python-aot/benchmark.json` records all timing samples,
versions, payload sizes, build time, and startup boundaries. Execution order
rotates between implementations. `--assert-target` on
`pybindings/bench_dataclasses_prepared.py` requires prepared elapsed time to be
at most 70% of Pydantic on **both** the small model and the recursive tree.
The large profile writes `target/python-aot/large/benchmark.json` and enforces
the same target independently for a 1,000-field model, a schema generating 201
classes, and 12,000 records (about 9.6 MB of JSON, including non-ASCII strings).
It also measures model import and the first round trip separately with the
runtime already imported. Both implementations enforce the emitted length,
numeric, required-property, and additional-property constraints; invalid
values at the end of each large payload are checked before timing.
To increase recursive depth, run the standard script with `--depth 7 --fanout 4
--tree-iterations 10 --output target/python-aot/deep/benchmark.json`.
Pydantic validates input; jsoncompat also validates output. These are measured
workloads, not a speed guarantee for every schema or machine. The test suite
differentially checks the prepared implementation against the ordinary runtime
across every generated fixture, supplied examples, and deterministic mutations.

Measured on macOS 26.6.2 arm64 with Python 3.12.2, Pydantic 2.13.4, and the
release extension (2026-10-08; median of nine rotating-order samples):

| Workload | JSON bytes | Prepared (µs) | Pydantic (µs) | Less elapsed time |
| --- | ---: | ---: | ---: | ---: |
| Small nested model | 228 | 2.437 | 3.924 | 37.9% |
| 1,365-node recursive tree | 268,847 | 2,107 | 3,366 | 37.4% |
| 21,845-node recursive tree | 4,350,138 | 36,039 | 63,408 | 43.2% |
| 1,000-field model | 65,391 | 269 | 390 | 31.0% |
| 201 generated classes | 159,352 | 661 | 1,074 | 38.4% |
| 12,000 records with Unicode strings | 9,622,824 | 47,000 | 70,116 | 33.0% |

With runtimes preloaded, model import for the 201-class schema took 10.8 ms
prepared, 96.8 ms with ordinary generated dataclasses, and 110.2 ms with
Pydantic. Preparation took 2.04 seconds out of band. On the small model,
preparation took 12.0 ms, model import took 0.39 ms, and its first round trip
took 47 µs (ordinary generated dataclasses: 1.22 ms import and 2.69 ms first
round trip). The standard report also includes full fresh-process timings so
interpreter and runtime startup are not hidden.

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
