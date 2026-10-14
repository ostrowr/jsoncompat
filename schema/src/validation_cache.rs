//! Reuse schema-validity proofs for unchanged definitions in generated namespaces.
//!
//! This never caches instance validation. A reused document still compiles its
//! complete original validator if a caller asks to validate an instance.
use crate::{SchemaDocument, compile, references};
use serde_json::{Map, Value};
use std::{collections::BTreeSet, sync::Arc};

/// A validity proof for the definitions of a previously validated namespace.
/// Only unchanged, unscoped definitions can be reused. There is no unchecked
/// constructor; obtain this from [`SchemaDocument::namespace_validation_cache`].
#[derive(Clone, Debug)]
pub struct SchemaValidationCache {
    dialect: Option<Value>,
    definitions: Arc<Map<String, Value>>,
}

impl SchemaValidationCache {
    pub(crate) fn from_document(document: &SchemaDocument) -> Option<Self> {
        let raw = document.source_schema_json();
        let object = raw.as_object()?;
        // Every definition must have been compiled as an ordinary property
        // schema. Unused $defs and branches skipped by backend optimizations do
        // not establish a proof of successful validator compilation.
        if object.get("type")?.as_str()? != "object"
            || object.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "$schema" | "$defs" | "type" | "properties" | "additionalProperties"
                )
            })
            || !plain_references(raw, &mut BTreeSet::new())
        {
            return None;
        }
        let definitions = object.get("$defs")?.as_object()?;
        let properties = object.get("properties")?.as_object()?;
        let exposed: BTreeSet<_> = properties
            .values()
            .filter_map(|value| {
                let object = value.as_object()?;
                (object.len() == 1).then_some(())?;
                reference_name(object.get("$ref")?.as_str()?)
            })
            .collect();
        if definitions.keys().any(|name| !exposed.contains(name)) {
            return None;
        }
        document.validate_source_schema().ok()?;
        Some(Self {
            dialect: object.get("$schema").cloned(),
            definitions: Arc::new(
                definitions
                    .iter()
                    .map(|(name, schema)| (name.clone(), without_declaration_metadata(schema)))
                    .collect(),
            ),
        })
    }

    pub(crate) fn validates(&self, raw: &Value) -> bool {
        if raw.get("$schema") != self.dialect.as_ref() {
            return false;
        }
        let mut references = BTreeSet::new();
        if !plain_references(raw, &mut references) {
            return false;
        }
        let Some(definitions) = raw.get("$defs").and_then(Value::as_object) else {
            return false;
        };
        if references
            .iter()
            .any(|name| !definitions.contains_key(name))
        {
            return false;
        }
        let mut reused = BTreeSet::new();
        for (name, schema) in definitions {
            if let Some(previous) = self.definitions.get(name) {
                if !matches_without_declaration_metadata(previous, schema) {
                    return false;
                }
                // A reused definition must retain its entire dependency closure
                // with exactly the meanings it had in the validated namespace.
                let mut dependencies = BTreeSet::new();
                if !plain_references(previous, &mut dependencies)
                    || dependencies.iter().any(|name| {
                        !self.definitions.contains_key(name) || !definitions.contains_key(name)
                    })
                {
                    return false;
                }
                reused.insert(name.as_str());
            }
        }
        if reused.is_empty() {
            return false;
        }
        // Do not deep-copy the proven definitions just to discard their copies.
        let remaining = Value::Object(
            raw.as_object()
                .expect("definitions object")
                .iter()
                .map(|(key, value)| {
                    let value = if key == "$defs" {
                        Value::Object(
                            definitions
                                .iter()
                                .map(|(name, schema)| {
                                    (
                                        name.clone(),
                                        if reused.contains(name.as_str()) {
                                            Value::Bool(true)
                                        } else {
                                            schema.clone()
                                        },
                                    )
                                })
                                .collect(),
                        )
                    } else {
                        value.clone()
                    };
                    (key.clone(), value)
                })
                .collect(),
        );
        // New wrappers and definitions are still checked by the same backend.
        // On failure the caller repeats the original path for exact diagnostics.
        compile(&remaining).is_ok()
    }
}

fn matches_without_declaration_metadata(previous: &Value, value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return previous == value;
    };
    if object
        .keys()
        .all(|key| matches!(key.as_str(), "allOf" | "x-jsoncompat"))
        && let Some(values) = object.get("allOf").and_then(Value::as_array)
        && values.len() == 1
        && values[0].is_boolean()
    {
        return previous == &values[0];
    }
    previous.as_object().is_some_and(|previous| {
        previous.len() == object.len() - usize::from(object.contains_key("x-jsoncompat"))
            && previous
                .iter()
                .all(|(key, value)| object.get(key) == Some(value))
    })
}

fn without_declaration_metadata(value: &Value) -> Value {
    let mut value = value.clone();
    if let Some(object) = value.as_object_mut() {
        object.remove("x-jsoncompat");
        // Code generation attaches declaration metadata to boolean definitions
        // by wrapping them in an allOf object.
        if object.len() == 1
            && let Some(values) = object.get("allOf").and_then(Value::as_array)
            && values.len() == 1
            && values[0].is_boolean()
        {
            return values[0].clone();
        }
    }
    value
}

fn reference_name(reference: &str) -> Option<String> {
    let encoded = reference.strip_prefix("#/$defs/")?;
    if encoded.contains(['/', '%']) {
        return None;
    }
    let name = encoded.replace("~1", "/").replace("~0", "~");
    (name.replace('~', "~0").replace('/', "~1") == encoded).then_some(name)
}

fn plain_references(schema: &Value, names: &mut BTreeSet<String>) -> bool {
    let Some(object) = schema.as_object() else {
        return schema.is_boolean();
    };
    if [
        "$id",
        "$anchor",
        "$dynamicRef",
        "$dynamicAnchor",
        "$recursiveRef",
        "$recursiveAnchor",
        "$vocabulary",
        "x-jsoncompat-format-assertion",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
    {
        return false;
    }
    if let Some(reference) = object.get("$ref") {
        let Some(name) = reference.as_str().and_then(reference_name) else {
            return false;
        };
        names.insert(name);
    }
    references::child_values(schema).all(|(_, child)| plain_references(child, names))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SchemaOptions;
    use serde_json::json;

    fn namespace() -> Value {
        json!({
            "$schema":"https://json-schema.org/draft/2020-12/schema",
            "type":"object", "properties":{
                "Node":{"$ref":"#/$defs/Node"}, "Reject":{"$ref":"#/$defs/Reject"}
            },
            "$defs":{
                "Node":{"type":"object", "properties":{
                    "id":{"type":"integer", "minimum":1},
                    "next":{"$ref":"#/$defs/Node"}
                }, "required":["id"], "additionalProperties":false},
                "Reject":false
            }
        })
    }

    fn compare(raw: &Value, cache: &SchemaValidationCache, options: &SchemaOptions) {
        let ordinary = SchemaDocument::from_json_with_options(raw, options);
        let reused = SchemaDocument::from_json_with_options_and_cache(raw, options, Some(cache));
        match (ordinary, reused) {
            (Ok(ordinary), Ok(reused)) => {
                assert_eq!(ordinary.source_schema_json(), reused.source_schema_json());
                assert_eq!(
                    ordinary.canonical_schema_json().unwrap(),
                    reused.canonical_schema_json().unwrap()
                );
                for value in [
                    json!(null),
                    json!({}),
                    json!({"Node":{"id":1}}),
                    json!({"Node":{"id":0}}),
                    json!({"Node":{"id":2,"next":{"id":3}}}),
                    json!({"Node":{"id":2,"next":{"id":0}}}),
                    json!({"Reject":null}),
                ] {
                    assert_eq!(
                        ordinary.is_valid(&value).map_err(|error| error.to_string()),
                        reused.is_valid(&value).map_err(|error| error.to_string()),
                        "{value}"
                    );
                }
            }
            (Err(ordinary), Err(reused)) => assert_eq!(ordinary.to_string(), reused.to_string()),
            _ => panic!("cached validation changed schema acceptance: {raw}"),
        }
    }

    #[test]
    fn borrowed_definition_comparison_matches_metadata_stripping() {
        let definitions = [
            json!(false),
            json!(true),
            json!({}),
            json!({"x-jsoncompat":{"name":"OnlyMetadata"}}),
            json!({"allOf":[false], "x-jsoncompat":{"name":"Reject"}}),
            json!({"allOf":[true]}),
            json!({"allOf":[true], "type":"object"}),
            json!({"allOf":[true, false]}),
            json!({"type":"object", "properties":{"a/b~c":{"type":"string"}}, "x-jsoncompat":{"name":"Record"}}),
            json!({"type":"object", "properties":{"a/b~c":{"type":"integer"}}}),
        ];
        for original in &definitions {
            let previous = without_declaration_metadata(original);
            for candidate in &definitions {
                assert_eq!(
                    matches_without_declaration_metadata(&previous, candidate),
                    previous == without_declaration_metadata(candidate),
                    "{original} versus {candidate}"
                );
            }
        }
    }

    #[test]
    fn unchanged_recursive_definitions_and_new_wrappers_reuse_validation() {
        let raw = namespace();
        let cache = SchemaDocument::from_json(&raw)
            .unwrap()
            .namespace_validation_cache()
            .unwrap();
        let mut generated = raw.clone();
        generated["$defs"]["Node"]["x-jsoncompat"] = json!({"name":"Node"});
        generated["$defs"]["Reject"] = json!({"allOf":[false], "x-jsoncompat":{"name":"Reject"}});
        generated["$defs"]["Response"] = json!({"type":"object", "properties":{
            "body":{"$ref":"#/$defs/Node"}, "status":{"enum":[200]}
        }});
        generated["properties"]["response"] = json!({"$ref":"#/$defs/Response"});
        assert!(cache.validates(&generated));
        compare(&generated, &cache, &SchemaOptions::default());
        // Skipping validation of the cached components must never skip their
        // constraints when is_valid subsequently compiles the original schema.
        let reused = SchemaDocument::from_json_with_options_and_cache(
            &generated,
            &SchemaOptions::default(),
            Some(&cache),
        )
        .unwrap();
        assert!(
            !reused
                .is_valid(&json!({"response":{"body":{"id":0},"status":200}}))
                .unwrap()
        );
    }

    #[test]
    fn changed_invalid_and_scope_sensitive_inputs_keep_the_original_checks() {
        let raw = namespace();
        let cache = SchemaDocument::from_json(&raw)
            .unwrap()
            .namespace_validation_cache()
            .unwrap();
        for replacement in [
            json!({"type":"string","pattern":"["}),
            json!({"type":"invalid"}),
            json!({"type":"integer","minimum":2}),
            json!({"$ref":"#/$defs/Missing"}),
            json!({"$ref":"#/$defs/Node/properties/id"}),
            json!({"$id":"https://example.test/node", "type":"string"}),
            json!({"$anchor":"node", "type":"integer"}),
        ] {
            let mut changed = raw.clone();
            changed["$defs"]["Node"] = replacement;
            assert!(!cache.validates(&changed));
            compare(&changed, &cache, &SchemaOptions::default());
        }
        for wrapper in [
            json!({"type":"string","pattern":"["}),
            json!({"$ref":"#/$defs/Missing"}),
            json!({"$ref":"#/$defs/Node/properties/id"}),
            json!({"minLength":-1}),
            json!({"$ref":"#/$defs/Node/x-jsoncompat"}),
        ] {
            let mut changed = raw.clone();
            changed["properties"]["wrapper"] = wrapper;
            assert!(!cache.validates(&changed));
            compare(&changed, &cache, &SchemaOptions::default());
        }
        let mut format = raw.clone();
        format["properties"]["email"] = json!({"type":"string","format":"email"});
        compare(
            &format,
            &cache,
            &SchemaOptions {
                assert_formats: Some(true),
                ..Default::default()
            },
        );
    }

    #[test]
    fn only_fully_exposed_unscoped_namespaces_establish_a_proof() {
        let raw = namespace();
        let mut unexposed = raw.clone();
        unexposed["properties"]
            .as_object_mut()
            .unwrap()
            .remove("Reject");
        let mut shortcut = raw.clone();
        shortcut["allOf"] = json!([false]);
        let mut scoped = raw.clone();
        scoped["$id"] = json!("https://example.test/root");
        for schema in [unexposed, shortcut, scoped] {
            assert!(
                SchemaDocument::from_json(&schema)
                    .unwrap()
                    .namespace_validation_cache()
                    .is_none()
            );
        }
        let cache = SchemaDocument::from_json(&raw)
            .unwrap()
            .namespace_validation_cache()
            .unwrap();
        let mut missing_dependency = raw;
        missing_dependency["$defs"]
            .as_object_mut()
            .unwrap()
            .remove("Node");
        assert!(!cache.validates(&missing_dependency));
    }
}
