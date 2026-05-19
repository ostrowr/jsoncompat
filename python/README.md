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

example = jsc.generate_value(old_schema, depth=5)
print(example)
```

## API

- `check_compat(old_schema_json: str, new_schema_json: str, role: str = "both") -> bool`
  - `role` must be `"serializer"`, `"deserializer"`, or `"both"`.
  - Raises `ValueError` for invalid schemas or hard unsupported compatibility features such as non-integral `number.multipleOf`.
- `generate_value(schema_json: str, depth: int = 5) -> str`
  - Returns a JSON string for one generated value accepted by the schema.
  - Raises `ValueError` when the schema is invalid, known to be unsatisfiable, or generation exhausts its retry budget.
- `Role.SERIALIZER`, `Role.DESERIALIZER`, and `Role.BOTH` are string constants accepted by `check_compat`.

Schemas are passed as JSON strings. `check_compat` returns a boolean verdict and raises `ValueError` for invalid JSON, invalid schemas, or hard unsupported compatibility cases.

## More detail

- [Basic demo](https://github.com/ostrowr/jsoncompat/blob/main/examples/python/basic/demo.py)
- https://jsoncompat.com
- [Repository README](https://github.com/ostrowr/jsoncompat/blob/main/readme.md)
- [Developer guide](https://github.com/ostrowr/jsoncompat/blob/main/developing.md)

## License

MIT License. See:

- https://github.com/ostrowr/jsoncompat/blob/main/LICENSE
