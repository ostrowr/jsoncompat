//! Cold diagnostics over the same prebuilt program used by the boolean evaluator.
//! No source schema, regular-expression compilation, or Python reflection is needed.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationFailureKind {
    Constraint,
    ResourceLimit,
    NonJson,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationFailure {
    pub kind: ValidationFailureKind,
    /// RFC 6901 pointer into the instance; empty means the root.
    pub instance_path: String,
    /// RFC 6901 pointer into the linked schema used to build this program.
    pub schema_path: String,
    pub keyword: String,
    pub message: String,
}

fn child(path: &str, key: &str) -> String {
    format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"))
}
fn failure(instance: &str, schema: &str, keyword: &str) -> ValidationFailure {
    ValidationFailure {
        kind: ValidationFailureKind::Constraint,
        instance_path: instance.into(),
        schema_path: if keyword == "false" {
            schema.into()
        } else {
            child(schema, keyword)
        },
        keyword: keyword.into(),
        message: format!("value does not satisfy {keyword}"),
    }
}

impl PreparedSchema {
    /// Explain the first failure. This deliberately performs extra work only on
    /// the error path; callers should keep using `is_valid_*` on successful input.
    pub fn explain_view<'a>(&self, value: impl InstanceView<'a>) -> Option<ValidationFailure> {
        if !value.is_json() {
            let mut result = failure("", "", "json");
            result.kind = ValidationFailureKind::NonJson;
            result.message = "value is outside the JSON data model".into();
            return Some(result);
        }
        // Owning the failed instance makes diagnostic traversal independent of
        // Python lifetimes and leaves the successful borrowed path unchanged.
        self.explain(&value.to_owned())
    }

    pub fn explain(&self, value: &Value) -> Option<ValidationFailure> {
        let mut context = Evaluation::default();
        let valid = self.accepts(NodeId(0), InstanceRef::from_serde(value), 0, &mut context);
        if context.incomplete {
            let mut result = failure("", "", "evaluation");
            result.kind = ValidationFailureKind::ResourceLimit;
            result.message =
                "prepared validation exceeded its depth or pattern evaluation budget".into();
            return Some(result);
        }
        if valid {
            return None;
        }
        self.explain_node(NodeId(0), value, "", "", &mut Vec::new())
    }

    fn explain_node(
        &self,
        id: NodeId,
        value: &Value,
        ip: &str,
        sp: &str,
        active: &mut Vec<(usize, String)>,
    ) -> Option<ValidationFailure> {
        let identity = (id.0, ip.to_owned());
        if active.contains(&identity) {
            return None;
        }
        active.push(identity);
        let result = self.explain_node_inner(id, value, ip, sp, active);
        active.pop();
        result
    }

    fn explain_node_inner(
        &self,
        id: NodeId,
        value: &Value,
        ip: &str,
        sp: &str,
        active: &mut Vec<(usize, String)>,
    ) -> Option<ValidationFailure> {
        let view = InstanceRef::from_serde(value);
        let node = &self.nodes[id.0];
        if node
            .types
            .as_ref()
            .is_some_and(|types| !types.iter().any(|kind| kind.accepts(view)))
        {
            return Some(failure(ip, sp, "type"));
        }
        if node
            .choices
            .as_ref()
            .is_some_and(|choices| !choices.iter().any(|choice| view.equals(choice)))
        {
            return Some(failure(
                ip,
                sp,
                if node.constant { "const" } else { "enum" },
            ));
        }
        for rule in &node.rules {
            let keyword = match rule {
                Rule::Ref { node, pointer } => {
                    if let Some(error) = self.explain_node(*node, value, ip, pointer, active) {
                        return Some(error);
                    }
                    continue;
                }
                Rule::All(nodes) => {
                    for (index, node) in nodes.iter().enumerate() {
                        if let Some(error) = self.explain_node(
                            *node,
                            value,
                            ip,
                            &child(&child(sp, "allOf"), &index.to_string()),
                            active,
                        ) {
                            return Some(error);
                        }
                    }
                    continue;
                }
                Rule::If {
                    condition,
                    then_node,
                    else_node,
                } => {
                    let (selected, key) =
                        if self.accepts(*condition, view, 0, &mut Evaluation::default()) {
                            (then_node, "then")
                        } else {
                            (else_node, "else")
                        };
                    if let Some(selected) = selected
                        && let Some(error) =
                            self.explain_node(*selected, value, ip, &child(sp, key), active)
                    {
                        return Some(error);
                    }
                    continue;
                }
                Rule::Object {
                    properties,
                    patterns,
                    required,
                    additional,
                } => {
                    let Some(object) = value.as_object() else {
                        continue;
                    };
                    if let Some(name) = required.iter().find(|name| !object.contains_key(*name)) {
                        let mut error = failure(ip, sp, "required");
                        error.message = format!("required property {name:?} is missing");
                        return Some(error);
                    }
                    for (key, value) in object {
                        let mut matched = false;
                        let path = child(ip, key);
                        if let Ok(index) = properties.binary_search_by(|(name, _)| name.cmp(key)) {
                            matched = true;
                            if let Some(error) = self.explain_node(
                                properties[index].1,
                                value,
                                &path,
                                &child(&child(sp, "properties"), key),
                                active,
                            ) {
                                return Some(error);
                            }
                        }
                        for (pattern, node) in patterns {
                            if self.patterns[pattern.0].is_match(key) == Some(true) {
                                matched = true;
                                if let Some(error) = self.explain_node(
                                    *node,
                                    value,
                                    &path,
                                    &child(
                                        &child(sp, "patternProperties"),
                                        &self.pattern_sources[pattern.0],
                                    ),
                                    active,
                                ) {
                                    return Some(error);
                                }
                            }
                        }
                        if !matched
                            && let Some(node) = additional
                            && let Some(error) = self.explain_node(
                                *node,
                                value,
                                &path,
                                &child(sp, "additionalProperties"),
                                active,
                            )
                        {
                            return Some(error);
                        }
                    }
                    continue;
                }
                Rule::Array { prefix, items } => {
                    let Some(array) = value.as_array() else {
                        continue;
                    };
                    for (index, item) in array.iter().enumerate() {
                        let (node, path) = if let Some(node) = prefix.get(index) {
                            (*node, child(&child(sp, "prefixItems"), &index.to_string()))
                        } else if let Some(node) = items {
                            (*node, child(sp, "items"))
                        } else {
                            continue;
                        };
                        if let Some(error) = self.explain_node(
                            node,
                            item,
                            &child(ip, &index.to_string()),
                            &path,
                            active,
                        ) {
                            return Some(error);
                        }
                    }
                    continue;
                }
                Rule::PropertyNames(node) => {
                    if let Some(object) = value.as_object() {
                        for key in object.keys() {
                            if let Some(error) = self.explain_node(
                                *node,
                                &Value::String(key.clone()),
                                &child(ip, key),
                                &child(sp, "propertyNames"),
                                active,
                            ) {
                                return Some(error);
                            }
                        }
                    }
                    continue;
                }
                Rule::DependentSchemas(entries) => {
                    if let Some(object) = value.as_object() {
                        for (key, node) in entries {
                            if object.contains_key(key)
                                && let Some(error) = self.explain_node(
                                    *node,
                                    value,
                                    ip,
                                    &child(&child(sp, "dependentSchemas"), key),
                                    active,
                                )
                            {
                                return Some(error);
                            }
                        }
                    }
                    continue;
                }
                Rule::UnevaluatedProperties(node) => {
                    if let Some(object) = value.as_object() {
                        for (key, item) in object {
                            if !self.evaluates(
                                id,
                                view,
                                annotations::Location::Property(key),
                                0,
                                &mut Evaluation::default(),
                                &mut Vec::new(),
                                false,
                            ) && let Some(error) = self.explain_node(
                                *node,
                                item,
                                &child(ip, key),
                                &child(sp, "unevaluatedProperties"),
                                active,
                            ) {
                                return Some(error);
                            }
                        }
                    }
                    continue;
                }
                Rule::UnevaluatedItems(node) => {
                    if let Some(array) = value.as_array() {
                        for (index, item) in array.iter().enumerate() {
                            if !self.evaluates(
                                id,
                                view,
                                annotations::Location::Item(index),
                                0,
                                &mut Evaluation::default(),
                                &mut Vec::new(),
                                false,
                            ) && let Some(error) = self.explain_node(
                                *node,
                                item,
                                &child(ip, &index.to_string()),
                                &child(sp, "unevaluatedItems"),
                                active,
                            ) {
                                return Some(error);
                            }
                        }
                    }
                    continue;
                }
                Rule::False => "false",
                Rule::Any(_) => "anyOf",
                Rule::One(_) => "oneOf",
                Rule::Not(_) => "not",
                Rule::Dependencies(_) => "dependentRequired",
                Rule::Contains { .. } => "contains",
                Rule::Unique => "uniqueItems",
                Rule::MultipleOf(_) => "multipleOf",
                Rule::Pattern(_) => "pattern",
                Rule::StringLength { min, .. } => {
                    if value
                        .as_str()
                        .is_some_and(|s| (s.chars().count() as u64) < *min)
                    {
                        "minLength"
                    } else {
                        "maxLength"
                    }
                }
                Rule::ArrayLength { min, .. } => {
                    if value.as_array().is_some_and(|a| (a.len() as u64) < *min) {
                        "minItems"
                    } else {
                        "maxItems"
                    }
                }
                Rule::ObjectLength { min, .. } => {
                    if value.as_object().is_some_and(|o| (o.len() as u64) < *min) {
                        "minProperties"
                    } else {
                        "maxProperties"
                    }
                }
                Rule::Bound {
                    lower, exclusive, ..
                } => match (lower, exclusive) {
                    (true, true) => "exclusiveMinimum",
                    (true, false) => "minimum",
                    (false, true) => "exclusiveMaximum",
                    (false, false) => "maximum",
                },
            };
            if !self.accepts_rule(rule, view, 0, &mut Evaluation::default()) {
                return Some(failure(ip, sp, keyword));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn diagnostic_locations_survive_interning_refs_and_annotations() {
        for (schema, value, ip, sp, keyword) in [
            (
                json!({"properties":{"first":{"type":"integer"},"a/b~":{"type":"array","items":{"type":"integer"}}}}),
                json!({"a/b~":[1,"bad"]}),
                "/a~1b~0/1",
                "/properties/a~1b~0/items/type",
                "type",
            ),
            (
                json!({"$defs":{"n":{"minimum":2}},"properties":{"x":{"$ref":"#/$defs/n"}}}),
                json!({"x":1}),
                "/x",
                "/$defs/n/minimum",
                "minimum",
            ),
            (
                json!({"anyOf":[{"properties":{"a":true}}],"unevaluatedProperties":false}),
                json!({"b":1}),
                "/b",
                "/unevaluatedProperties",
                "false",
            ),
            (
                json!({"prefixItems":[true],"unevaluatedItems":false}),
                json!([1, 2]),
                "/1",
                "/unevaluatedItems",
                "false",
            ),
            (
                json!({"patternProperties":{"^x/":{"const":2}}}),
                json!({"x/a":3}),
                "/x~1a",
                "/patternProperties/^x~1/const",
                "const",
            ),
            (
                json!({"if":{"type":"string"},"then":{"minLength":3},"else":{"minimum":4}}),
                json!(2),
                "",
                "/else/minimum",
                "minimum",
            ),
        ] {
            let program = PreparedSchema::load(
                &PreparedSchema::compile(&schema)
                    .unwrap()
                    .to_bytes()
                    .unwrap(),
            )
            .unwrap();
            let error = program.explain(&value).unwrap();
            assert_eq!(
                (&*error.instance_path, &*error.schema_path, &*error.keyword),
                (ip, sp, keyword)
            );
            assert_eq!(error.kind, ValidationFailureKind::Constraint);
        }
    }
}
