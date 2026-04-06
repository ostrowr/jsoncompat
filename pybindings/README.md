# jsoncompat

Python bindings for checking compatibility of evolving JSON schemas and generating example values.

## Installation

Install from PyPI:

```bash
pip install jsoncompat==0.3.1
```

## Usage

```python
import jsoncompat as jsc

# Define old and new schemas as JSON strings
old_schema = '{"type": "string"}'
new_schema = '{"type": "number"}'

# Check compatibility (role: "serializer", "deserializer", or "both").
# Raises ValueError for invalid schemas or unsupported compatibility features.
is_compatible = jsc.check_compat(old_schema, new_schema, jsc.Role.BOTH)
print(is_compatible)

# Generate example values for a schema
generator = jsc.generator_for(old_schema)
example = generator.generate_value(depth=5)
print(example)
```

## Public Interface

- `check_compat(old_schema_json: str, new_schema_json: str, role: str = "both") -> bool`
  - `role` must be `"serializer"`, `"deserializer"`, or `"both"`.
  - Raises `ValueError` for invalid schemas or unsupported compatibility features such as non-integral `number.multipleOf`.
- `generator_for(schema_json: str) -> Generator`
  - Parses the schema once and returns a reusable generator.
  - `Generator.generate_value(depth: int = 5) -> str` returns a JSON string for one generated value accepted by the schema.
  - Raises `ValueError` when the schema is invalid, known to be unsatisfiable, or generation exhausts its retry budget.
- `validator_for(schema_json: str) -> Validator`
  - Parses the schema once and returns a reusable validator.
  - `Validator.is_valid(instance_json: str) -> bool` validates JSON strings against the parsed schema.
- `generate_value(schema_json: str, depth: int = 5) -> str`
  - Deprecated. Use `generator_for(schema_json).generate_value(depth)` instead.
- `is_valid(schema_json: str, instance_json: str) -> bool`
  - Deprecated. Use `validator_for(schema_json).is_valid(instance_json)` instead.
- `jsoncompat.codegen.dataclasses` runtime helpers for generated dataclass models
- `Role.SERIALIZER`, `Role.DESERIALIZER`, and `Role.BOTH` are string constants accepted by `check_compat`.

Schemas are passed as JSON strings. The bindings are intentionally thin: they
parse the strings, call the Rust core APIs, and map Rust errors into
`ValueError`.

## Examples

See the basic demo:

- https://github.com/ostrowr/jsoncompat/blob/main/examples/python/basic/demo.py
- https://jsoncompat.com

## Benchmarks

Run the generated dataclass runtime microbenchmark from the repository root:

```bash
env -u VIRTUAL_ENV uv run --project pybindings python pybindings/bench_dataclasses_runtime.py
```

## License

MIT License. See:

- https://github.com/ostrowr/jsoncompat/blob/main/LICENSE
