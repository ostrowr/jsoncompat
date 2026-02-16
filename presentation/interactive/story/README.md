# Story JSON Format

`default-story.json` defines schema versions, left/right state pairings, and transition order.

## Shape

```json
{
  "versions": [{ "id": "v1", "schema": { "type": "object", "properties": {}, "required": [] } }],
  "states": [{ "id": "s1", "leftVersionId": "v1", "rightVersionId": "v1" }],
  "transitions": [{ "id": "t1", "fromStateId": "s1", "toStateId": "s2", "seedWireFrom": "left_before" }],
  "initialStateId": "s1"
}
```

## Notes

- `versions[].schema` supports a focused JSON Schema subset:
  - objects with `properties` + `required`
  - scalar fields (`string`, `integer`, `number`, `boolean`, `null`)
  - arrays with `items`
  - nullable unions via `type: ["<type>", "null"]`
- `seedWireFrom` is validated and reserved for future transition seeding policies.
- Exactly one outgoing transition per state is currently supported in runtime validation.
