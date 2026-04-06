# jsoncompat (JavaScript/WebAssembly)

WebAssembly bindings for checking compatibility of evolving JSON schemas and
generating example values from JavaScript.

## Installation

```bash
npm install jsoncompat@0.3.1
```

## Public Interface

```js
import init, { check_compat, generate_value } from "jsoncompat";

await init();

const oldSchema = '{"type":"string"}';
const newSchema = '{"type":["string","null"]}';

const ok = check_compat(oldSchema, newSchema, "deserializer");
const valueJson = generate_value(newSchema, 5);
```

- `check_compat(old_schema_json, new_schema_json, role) -> boolean`
  accepts `"serializer"`, `"deserializer"`, or `"both"` for `role`.
- `generate_value(schema_json, depth) -> string` returns one generated JSON value
  encoded as a string.

Both functions accept schemas as JSON strings and throw string-backed
`wasm-bindgen` errors for invalid JSON, invalid schemas, unsupported
compatibility features, known-unsatisfiable schemas, or retry exhaustion.

For full documentation and examples, see:

- https://jsoncompat.com
- https://github.com/ostrowr/jsoncompat

## License

MIT
