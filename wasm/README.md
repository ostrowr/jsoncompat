# jsoncompat (JavaScript/WebAssembly)

WebAssembly bindings for checking compatibility of evolving JSON Schemas and
generating example values from JavaScript.

## Installation

```bash
npm install jsoncompat@0.3.1
```

## Quick start

```js
import init, { check_compat, generate_value } from "jsoncompat";

await init();

const oldSchema = '{"type":"string"}';
const newSchema = '{"type":["string","null"]}';

const ok = check_compat(oldSchema, newSchema, "deserializer");
const valueJson = generate_value(newSchema, 5);
```

## API

- `check_compat(old_schema_json, new_schema_json, role) -> boolean`
  accepts `"serializer"`, `"deserializer"`, or `"both"` for `role`.
- `generate_value(schema_json, depth) -> string` returns one generated JSON value
  encoded as a string.

Both functions accept schemas as JSON strings and throw string-backed
`wasm-bindgen` errors for invalid JSON, invalid schemas, hard unsupported
compatibility features, known-unsatisfiable schemas, or retry exhaustion.

## More detail

- https://jsoncompat.com
- https://github.com/ostrowr/jsoncompat
- [Repository README](https://github.com/ostrowr/jsoncompat/blob/main/readme.md)
- [Developer guide](https://github.com/ostrowr/jsoncompat/blob/main/developing.md)

## License

MIT
