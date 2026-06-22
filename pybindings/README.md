# jsoncompat

Python bindings for checking compatibility of evolving JSON Schemas and generating example values.

## Installation

Install from PyPI:

```bash
pip install jsoncompat==0.3.1
```

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

See the [canonical plain-schema example](../examples/dataclasses/demo.py) for an
ordinary generated model that both serializes and deserializes. The
[canonical stamped-schema example](../examples/stamp/demo.py) covers versioned
writer/reader envelopes and historical schemas.

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

For a larger workload, benchmark a balanced recursive graph containing
discriminated unions, lists, mappings, and omitted fields:

```bash
just python-bench-scale
```

The default depth and fanout produce 1,365 model nodes. Override them, along
with the iteration and repeat counts, as positional recipe arguments. The
underlying script also accepts `--profile` to print cumulative Python profiles
for checked and trusted JSON deserialization.

To cover the complete checked-in JSON Schema fixture corpus, generate a
candidate Pydantic v2 peer with the pinned `datamodel-code-generator` version
and benchmark every pair that passes the equivalence screen:

```bash
just python-bench-fixtures
```

This includes every embedded fuzz schema and both sides of every backcompat
fixture. Generated Pydantic modules and the detailed JSON report are written to
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
