# jsoncompat

JSON Schema Compatibility Checker Python Bindings

Check compatibility of evolving JSON schemas and generate example values using Python.

## Installation

Install from PyPI:

```bash
pip install jsoncompat
```

## Usage

```python
import jsoncompat as jsc

# Define old and new schemas as JSON strings
old_schema = '{"type": "string"}'
new_schema = '{"type": "number"}'

# Check compatibility (role: "serializer", "deserializer", or "both")
is_compatible = jsc.check_compat(old_schema, new_schema, "both")
print(is_compatible)

# Generate an example value for a schema
example = jsc.generate_value(old_schema, depth=5)
print(example)
```

## API Reference

- `check_compat(old_schema_json: str, new_schema_json: str, role: str = "both") -> bool`
- `generate_value(schema_json: str, depth: int = 5) -> str`
  - Raises `ValueError` when the schema has no valid instances or no example can be generated at the requested depth.

## Examples

See the [basic demo](../examples/python/basic/demo.py) for a complete end-to-end script.

## License

MIT License. See [LICENSE](../LICENSE).
