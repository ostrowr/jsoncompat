# jsoncompat (JavaScript/WebAssembly)

WebAssembly bindings for checking compatibility of evolving JSON schemas and
generating example values from JavaScript.

## Installation

```bash
npm install jsoncompat@0.3.1
```

## Public Interface

```js
import init, { check_compat, generator_for, validator_for } from "jsoncompat";

await init();

const oldSchema = '{"type":"string"}';
const newSchema = '{"type":["string","null"]}';

const ok = check_compat(oldSchema, newSchema, "deserializer");
const generator = generator_for(newSchema);
const valueJson = generator.generate_value(5);
const validator = validator_for(newSchema);
const valueOk = validator.is_valid(valueJson);
```

- `check_compat(old_schema_json, new_schema_json, role) -> boolean`
  accepts `"serializer"`, `"deserializer"`, or `"both"` for `role`.
- `generator_for(schema_json) -> Generator` parses a schema once and returns a
  reusable generator.
  - `Generator.generate_value(depth) -> string` returns one generated JSON value
    encoded as a string.
- `validator_for(schema_json) -> Validator` parses a schema once and returns a
  reusable validator.
  - `Validator.is_valid(instance_json) -> boolean` validates a JSON string
    against the parsed schema.
- `generate_value(schema_json, depth) -> string` is kept for compatibility. Prefer
  `generator_for(schema_json).generate_value(depth)`.

Functions accept schemas as JSON strings and throw string-backed `wasm-bindgen`
errors for invalid JSON, invalid schemas, unsupported compatibility features,
known-unsatisfiable schemas, or retry exhaustion.

For full documentation and examples, see:

- https://jsoncompat.com
- https://github.com/ostrowr/jsoncompat

## License

MIT
