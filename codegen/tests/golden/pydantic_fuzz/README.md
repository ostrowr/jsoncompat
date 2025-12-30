# Pydantic fuzz goldens

Runtime validation for the generated Pydantic serializers that correspond to `tests/fixtures/fuzz`.
Goldens and test configuration live in this directory.

Usage:

```bash
cd codegen/tests/golden/pydantic_fuzz
uv run pytest
```

Set `REGEN_CODEGEN_GOLDENS=1` and rerun the Rust `fuzz_codegen` test to refresh goldens before
running pytest.
