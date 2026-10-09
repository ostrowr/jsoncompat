//! Caller-supplied resources and Draft 2020-12 format vocabulary selection.
use crate::{AstError, references};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

pub(crate) const ASSERT_FORMAT: &str = "x-jsoncompat-format-assertion";
const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";
const VOCAB: &str = "https://json-schema.org/draft/2020-12/vocab/";

/// Resource retrieval is explicit and offline. Format assertions default to the
/// vocabulary selected by `$schema`; standard Draft 2020-12 formats annotate.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SchemaOptions {
    pub resources: BTreeMap<String, Value>,
    /// Override vocabulary selection for every resource when `Some`.
    pub assert_formats: Option<bool>,
}

pub(crate) fn prepare(raw: &Value, options: &SchemaOptions) -> Result<Value, AstError> {
    let mut root = prepare_node(raw, false, options)?;
    if options.resources.is_empty() {
        return Ok(root);
    }
    if root.is_boolean() {
        root = json!({"allOf":[root]});
    }
    let object = root
        .as_object_mut()
        .ok_or_else(|| AstError::InvalidOptions {
            reason: "root schema must be an object or boolean".into(),
        })?;
    object
        .entry("$id")
        .or_insert_with(|| json!("https://jsoncompat.invalid/root"));
    let definitions = object
        .entry("$defs")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| AstError::InvalidOptions {
            reason: "$defs must be an object".into(),
        })?;
    for (index, (uri, resource)) in options.resources.iter().enumerate() {
        let base = url::Url::parse(uri).map_err(|_| AstError::InvalidOptions {
            reason: format!("resource URI must be absolute: {uri}"),
        })?;
        let mut resource = prepare_node(resource, false, options)?;
        if resource.is_boolean() {
            resource = json!({"allOf":[resource]});
        }
        let object = resource
            .as_object_mut()
            .ok_or_else(|| AstError::InvalidOptions {
                reason: format!("resource {uri} must be an object or boolean"),
            })?;
        let id = object
            .get("$id")
            .and_then(Value::as_str)
            .map_or_else(|| Ok(base.clone()), |id| base.join(id))
            .map_err(|_| AstError::InvalidOptions {
                reason: format!("invalid resource identifier in {uri}"),
            })?
            .to_string();
        object.insert("$id".into(), json!(id));
        insert_unique(definitions, format!("__resource_{index}"), resource);
        if id != *uri {
            insert_unique(
                definitions,
                format!("__alias_{index}"),
                json!({"$id":uri,"$ref":id}),
            );
        }
    }
    Ok(root)
}

fn insert_unique(definitions: &mut Map<String, Value>, mut name: String, value: Value) {
    while definitions.contains_key(&name) {
        name.push('_');
    }
    definitions.insert(name, value);
}

fn prepare_node(
    schema: &Value,
    inherited: bool,
    options: &SchemaOptions,
) -> Result<Value, AstError> {
    let mut result = schema.clone();
    let Some(object) = result.as_object_mut() else {
        return Ok(result);
    };
    let mut asserted = inherited;
    if let Some(uri) = object.get("$schema").and_then(Value::as_str) {
        if let Some(meta) = options.resources.get(uri) {
            let vocabulary = meta
                .get("$vocabulary")
                .and_then(Value::as_object)
                .ok_or_else(|| AstError::InvalidOptions {
                    reason: format!(
                        "custom dialect {uri} must declare its Draft 2020-12 vocabularies"
                    ),
                })?;
            let core = format!("{VOCAB}core");
            if vocabulary.get(&core) != Some(&json!(true)) {
                return Err(AstError::InvalidOptions {
                    reason: format!("custom dialect {uri} must require Draft 2020-12 core"),
                });
            }
            for (name, required) in vocabulary {
                if !required.is_boolean() {
                    return Err(AstError::InvalidOptions {
                        reason: format!("vocabulary requirement for {name} must be boolean"),
                    });
                }
                if required == &json!(true)
                    && ![
                        "core",
                        "applicator",
                        "unevaluated",
                        "validation",
                        "meta-data",
                        "format-annotation",
                        "format-assertion",
                        "content",
                    ]
                    .iter()
                    .any(|suffix| name == &format!("{VOCAB}{suffix}"))
                {
                    return Err(AstError::InvalidOptions {
                        reason: format!("unsupported required vocabulary {name}"),
                    });
                }
            }
            asserted = vocabulary.contains_key(&format!("{VOCAB}format-assertion"));
            object.insert("$schema".into(), json!(DRAFT));
        } else if uri.trim_end_matches('#') == DRAFT
            || uri == "https://spec.openapis.org/oas/3.1/dialect/base"
        {
            asserted = false;
        }
    }
    if options.assert_formats.unwrap_or(asserted)
        && let Some(format) = object.get("format").cloned()
    {
        object.insert(ASSERT_FORMAT.into(), format);
    }
    for (suffix, child) in references::children(schema) {
        let prepared = prepare_node(child, asserted, options)?;
        if let Some(target) = result.pointer_mut(&format!("/{suffix}")) {
            *target = prepared;
        }
    }
    Ok(result)
}

pub(crate) fn has_assertions(schema: &Value) -> bool {
    schema.get(ASSERT_FORMAT).is_some()
        || references::children(schema)
            .iter()
            .any(|(suffix, child)| suffix != "contentSchema" && has_assertions(child))
}

pub(crate) fn configure(options: jsonschema::ValidationOptions) -> jsonschema::ValidationOptions {
    options.with_keyword(ASSERT_FORMAT, |_, format, _| {
        let validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .should_ignore_unknown_formats(false)
            .build(&json!({"format":format}))
            .map_err(|error| error.to_owned())?;
        Ok(Box::new(AssertedFormat(validator)))
    })
}
struct AssertedFormat(jsonschema::Validator);
impl jsonschema::Keyword for AssertedFormat {
    fn is_valid(&self, instance: &Value) -> bool {
        self.0.is_valid(instance)
    }
    fn validate<'i>(&self, instance: &'i Value) -> Result<(), jsonschema::ValidationError<'i>> {
        self.0.validate(instance)
    }
}
