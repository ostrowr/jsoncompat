import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import init, { analyze_compat } from '../pkg-web/jsoncompat_wasm.js';

await init({ module_or_path: await readFile(new URL('../pkg-web/jsoncompat_wasm_bg.wasm', import.meta.url)) });
assert.equal(analyze_compat('{"type":"integer"}', '{"type":"integer"}', 'both').status, 'compatible');
const broken = analyze_compat('{"type":"string"}', '{"type":"integer"}', 'serializer');
assert.equal(broken.status, 'incompatible');
assert.equal(broken.direction, 'serializer');
assert.equal(typeof JSON.parse(broken.counterexample_json), 'number');
const properties = Object.fromEntries(Array.from({ length: 9 }, (_, i) => [`p${i}`, true]));
const simple = { type: 'object', properties, additionalProperties: false };
const complex = { type: 'object', anyOf: Object.keys(properties).map(key => ({ properties: { [key]: true } })), unevaluatedProperties: false };
const unknown = analyze_compat(JSON.stringify(simple), JSON.stringify(complex), 'both');
assert.equal(unknown.status, 'unknown');
assert.equal(typeof unknown.reason, 'string');
console.log('Structured WASM verdicts verified');
