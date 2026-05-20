# Generated dataclass fixtures

These snapshots are generated from every checked-in sample schema under
`tests/fixtures/backcompat`, `tests/fixtures/fuzz`, and
`examples/stamp/schema-v1.json` + `examples/stamp/schema-v2.json`.

Successful generations are stored as `.py` files. Generator failures are stored
as `.error.txt` snapshots with the exact diagnostic.

Regenerate everything with:

```bash
just regen-dataclasses-fixtures
```
