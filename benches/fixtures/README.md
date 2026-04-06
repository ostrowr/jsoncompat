# Benchmark Fixtures

Each `*.json` file in this directory defines one stable benchmark schema and
one known-valid instance:

```json
{
  "schema": { "type": "string" },
  "instance": "example"
}
```

Keep this corpus small, intentional, and stable. Adding or rewriting these
fixtures changes the benchmark baseline, so prefer one file per hotspot and do
not mirror the broader fuzz corpus here.
