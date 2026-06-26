# Generated dataclasses from a JSON Schema

This is the canonical Python codegen example for an ordinary JSON Schema. It
does not use `jsoncompat stamp`, a version envelope, or separate reader and
writer models.

Regenerate the importable model module:

```bash
jsoncompat codegen --target dataclasses examples/dataclasses/schema.json > examples/dataclasses/models.py
```

Run the end-to-end example:

```bash
uv run examples/dataclasses/demo.py
```

The demo covers checked construction, nested generated types, Python JSON
values, JSON/YAML/MessagePack round trips, omitted versus explicit `null`, the
trusted path, and invalid-input rejection.
