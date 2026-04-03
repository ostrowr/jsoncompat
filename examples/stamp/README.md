# Stamped schema example

This example shows a schema history manifest, a breaking schema revision, and
the generated stamped bundle plus standalone writer/reader schema codegen.

```bash
jsoncompat stamp --manifest examples/stamp/manifest.json --id user-profile --write-manifest examples/stamp/schema-v2.json
jsoncompat stamp --manifest examples/stamp/manifest.json --id user-profile --display writer examples/stamp/schema-v2.json > examples/stamp/writer.schema.json
jsoncompat stamp --manifest examples/stamp/manifest.json --id user-profile --display reader examples/stamp/schema-v2.json > examples/stamp/reader.schema.json
jsoncompat codegen --target dataclasses examples/stamp/writer.schema.json > examples/stamp/writer.dataclasses.py
jsoncompat codegen --target dataclasses examples/stamp/reader.schema.json > examples/stamp/reader.dataclasses.py
```
