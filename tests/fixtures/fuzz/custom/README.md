# Custom fuzz fixtures

These fixtures are jsoncompat-specific regression cases. They are meant to cover tricky canonicalization, AST construction, and fuzz-generation behavior that is not directly represented by the vendored JSON Schema Test Suite files in the parent directory.

## Format

Each file is a JSON array of fixture groups:

```json
[
  {
    "description": "human-readable schema description",
    "schema": {
      "type": "string"
    },
    "tests": [
      {
        "description": "accepted example",
        "data": "ok",
        "valid": true
      }
    ]
  }
]
```

## Test behavior

- `tests/fuzz.rs` builds a canonical AST from each `schema`, generates random candidate instances from that AST, and validates them against the original raw schema.
- For fixtures under this `custom/` directory, every entry in `tests` is also executed against the raw schema validator as an explicit regression assertion.
- For vendored fixtures outside `custom/`, the harness fuzzes generated candidates but does not treat every upstream example in `tests` as a required pass/fail assertion, because many optional or unsupported spec behaviors are intentionally outside this crate's current scope.

## When to add a fixture

Add a custom fixture when a bug depends on one of these patterns:

- local `$ref` targets that canonicalization might prune or renumber
- recursive or mutually recursive `$defs`
- `enum`/`const` combined with type-specific keywords
- integer-boundary normalization and `contains` cardinality
- `if` / `then` / `else`, boolean applicators, or metadata preservation
- unknown-keyword branches that should survive because they are locally referenced
