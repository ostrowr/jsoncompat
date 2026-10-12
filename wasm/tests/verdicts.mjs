import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const precise = process.argv.includes('--arbitrary-precision');
const packageDirectory = precise ? '../pkg-precise/' : '../pkg-web/';
const { default: init, analyze_compat, validator_for, generator_for } = await import(
  new URL(`${packageDirectory}jsoncompat_wasm.js`, import.meta.url)
);
await init({ module_or_path: await readFile(new URL(`${packageDirectory}jsoncompat_wasm_bg.wasm`, import.meta.url)) });

assert.equal(validator_for('{"const":0.10000000000000001}').is_valid('0.1'), !precise);
assert.equal(validator_for('{"const":0}').is_valid('1e-400'), !precise);
assert.equal(validator_for('{"const":18446744073709551615}').is_valid('18446744073709551615'), true);
if (precise) {
  assert.equal(validator_for('true').is_valid('1e400'), true);
} else {
  assert.throws(() => validator_for('true').is_valid('1e400'));
}
console.log(`WASM numeric precision verified: ${precise ? 'opt-in' : 'default'}`);
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

const schema = '{"$ref":"https://example.com/name"}';
const options = JSON.stringify({resources: {"https://example.com/name": {type: "string", minLength: 2}}});
const validator = validator_for(schema, options);
assert.equal(validator.is_valid('"ok"'), true);
assert.equal(validator.is_valid('"x"'), false);
assert.equal(validator.is_valid(generator_for(schema, options).generate_value(3)), true);
assert.equal(analyze_compat(schema, schema, 'both', options).status, 'compatible');
assert.equal(validator_for('{"format":"email"}', '{"assert_formats":true}').is_valid('"invalid"'), false);
assert.equal(validator_for('{"format":"email"}', '{"assert_formats":false}').is_valid('"invalid"'), true);
