> **ℹ️ See [jsoncompat.com](https://jsoncompat.com) for more information.**

# jsoncompat

Check compatibility of evolving JSON schemas.

Imagine you have an API that returns some JSON data, or JSON that you're storing in a database or file. You need to ensure that new code can read old data and that old code can read new data.

It's difficult to version JSON schemas in a traditional sense, because they can break in two directions:

1. If a schema is used by the party generating the data, or "serializer", then a change to the schema that can break clients using an older version of the schema should be considered "breaking." For example, removing a required property from a serializer schema should be considered a breaking change for a schema with the serializer role.

More formally, consider a serializer schema $S_A$ which is changed to $S_B$. This change should be considered breaking if there exists some JSON value that is valid against $S_B$ but invalid against $S_A$.

As a concrete example, if you're a webserver that returns JSON data with the following schema:

```json
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" },
    "name": { "type": "string" }
  },
  "required": ["id", "name"]
}
```

and you make `name` optional:

```json
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" },
    "name": { "type": "string" }
  },
  "required": ["id"]
}
```

then you've made a breaking change for any client that is using the old schema.

We assume that the serializer will not write additional properties that are not in the schema, even if additionalProperties is true. This allows us to consider a change to the schema that adds an optional property of some type not to be a breaking change.

1. If a schema is used by a party receiving the data, or "deserializer", then a change to the schema that might fail to deserialize existing data should be considered "breaking." For example, adding a required property to a deserializer should be considered a breaking change.

More formally, consider a deserializer schema $S_A$ which is changed to $S_B$. This change should be considered breaking if there exists some JSON value that is valid against $S_A$ but invalid against $S_B$.

As a concrete example, imagine that you've been writing code that saves JSON data to a databse with the following schema:

```json
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" },
    "name": { "type": "string" }
  },
  "required": ["id"]
}
```

and you make `name` required, attempting to load that data into memory by deserializing it with the following schema:

```json
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" },
    "name": { "type": "string" }
  },
  "required": ["id", "name"]
}
```

you'll be unable to deserialize any data that doesn't have a `name` property, which is a breaking change for the `deserializer` role.

If a schema is used by both a serializer and a deserializer, then a change to the schema that can break either should be considered "breaking."
