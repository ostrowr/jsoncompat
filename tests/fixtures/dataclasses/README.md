# Generated dataclass fixtures

The full corpus is generated during tests from the schemas under
`tests/fixtures/backcompat`, `tests/fixtures/fuzz`, `examples/stamp`, and
`pybindings/benchmark_schemas`. Every supported schema still runs through
syntax checks, runtime validation, and round-trip differential tests.

This directory keeps representative benchmark/example Python goldens and exact
`.error.txt` diagnostics for unsupported inputs. Hundreds of mechanically
regenerable Python copies are deliberately omitted. The classification manifests
remain checked in and independently tested.

```bash
just regen-dataclasses-fixtures
cargo run --example dataclass_corpus -- target/generated-dataclass-fixtures
```

The first command updates representative goldens, error diagnostics, and the
split public examples. The second writes the full corpus for fixture benchmarks.
