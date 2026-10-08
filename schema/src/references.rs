//! Resolve resource identifiers and dynamic scope into ordinary local references.
//! This path is used only for resource-aware documents; plain local pointers keep
//! the existing lightweight resolver. No network retrieval happens here.
use crate::{
    AstError, SCHEMA_ARRAY_CHILD_KEYWORDS, SCHEMA_MAP_CHILD_KEYWORDS, SINGLE_SCHEMA_CHILD_KEYWORDS,
};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use url::Url;

const ROOT_URI: &str = "https://jsoncompat.invalid/root";
const MAX_INSTANTIATIONS: usize = 4096;

struct Entry<'a> {
    value: &'a Value,
    pointer: String,
    resource: String,
}

#[derive(Default)]
struct Linker<'a> {
    entries: Vec<Entry<'a>>,
    pointers: HashMap<String, usize>,
    resources: HashMap<String, usize>,
    anchors: HashMap<String, usize>,
    instances: HashMap<(usize, Vec<String>), usize>,
    definitions: Map<String, Value>,
}

pub(crate) fn needs_linking(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.keys().any(|key| {
        matches!(
            key.as_str(),
            "$id" | "$anchor" | "$dynamicRef" | "$dynamicAnchor"
        )
    }) || children(value)
        .iter()
        .any(|(_, child)| needs_linking(child))
}

pub(crate) fn link(schema: &Value) -> Result<Value, AstError> {
    fn flat(schema: &Value) -> bool {
        schema
            .get("$ref")
            .and_then(Value::as_str)
            .is_none_or(|reference| {
                reference
                    .strip_prefix("#/$defs/")
                    .is_some_and(|name| !name.contains('/'))
            })
            && children(schema).iter().all(|(_, child)| flat(child))
    }
    // Already linked documents are a stable serialization of the graph.
    if schema.as_object().is_some_and(|object| {
        object.len() == 2 && object.contains_key("$ref") && object.contains_key("$defs")
    }) && !needs_linking(schema)
        && flat(schema)
    {
        return Ok(schema.clone());
    }
    let mut linker = Linker::default();
    linker.index(schema, "#".into(), ROOT_URI)?;
    let root = linker.instantiate(0, &[])?;
    Ok(json!({"$ref":root, "$defs":linker.definitions}))
}

fn resolve(base: &str, reference: &str) -> Result<String, AstError> {
    let uri = Url::parse(base)
        .and_then(|base| base.join(reference))
        .map_err(|_| AstError::UnsupportedReference {
            ref_path: reference.to_owned(),
        })?;
    let mut uri = uri;
    if uri.fragment() == Some("") {
        uri.set_fragment(None);
    }
    Ok(uri.to_string())
}

impl<'a> Linker<'a> {
    fn index(
        &mut self,
        value: &'a Value,
        pointer: String,
        inherited: &str,
    ) -> Result<(), AstError> {
        let resource = value
            .get("$id")
            .and_then(Value::as_str)
            .map(|id| resolve(inherited, id))
            .transpose()?
            .unwrap_or_else(|| inherited.to_owned());
        let index = self.entries.len();
        self.entries.push(Entry {
            value,
            pointer: pointer.clone(),
            resource: resource.clone(),
        });
        self.pointers.insert(pointer.clone(), index);
        if pointer == "#" {
            self.resources.insert(ROOT_URI.into(), index);
        }
        if (pointer == "#" || value.get("$id").is_some())
            && let Some(previous) = self.resources.insert(resource.clone(), index)
            && previous != index
        {
            return Err(AstError::AmbiguousResource { uri: resource });
        }
        for keyword in ["$anchor", "$dynamicAnchor"] {
            if let Some(anchor) = value.get(keyword).and_then(Value::as_str) {
                let uri = resolve(&resource, &format!("#{anchor}"))?;
                if let Some(previous) = self.anchors.insert(uri.clone(), index)
                    && previous != index
                {
                    return Err(AstError::AmbiguousResource { uri });
                }
            }
        }
        for (suffix, child) in children(value) {
            self.index(child, format!("{pointer}/{suffix}"), &resource)?;
        }
        Ok(())
    }

    fn lookup(&mut self, base: &str, reference: &str) -> Result<usize, AstError> {
        let uri = resolve(base, reference)?;
        if let Some(index) = self.anchors.get(&uri) {
            return Ok(*index);
        }
        let mut parsed = Url::parse(&uri).expect("resolved URI");
        let fragment = parsed.fragment().unwrap_or("").to_owned();
        parsed.set_fragment(None);
        let root =
            self.resources
                .get(parsed.as_str())
                .ok_or_else(|| AstError::UnresolvedReference {
                    ref_path: uri.clone(),
                })?;
        if fragment.is_empty() {
            return Ok(*root);
        }
        let fragment = percent_encoding::percent_decode_str(&fragment)
            .decode_utf8()
            .map_err(|_| AstError::UnresolvedReference {
                ref_path: uri.clone(),
            })?;
        if !fragment.starts_with('/') {
            return Err(AstError::UnresolvedReference { ref_path: uri });
        }
        let pointer = format!("{}{}", self.entries[*root].pointer, fragment);
        if let Some(index) = self.pointers.get(&pointer) {
            return Ok(*index);
        }
        let value = self.entries[*root]
            .value
            .pointer(&fragment)
            .filter(|value| value.is_object() || value.is_boolean())
            .ok_or_else(|| AstError::UnresolvedReference {
                ref_path: uri.clone(),
            })?;
        let resource = self.entries[*root].resource.clone();
        let index = self.entries.len();
        self.index(value, pointer, &resource)?;
        Ok(index)
    }

    fn instantiate(&mut self, index: usize, incoming: &[String]) -> Result<String, AstError> {
        let resource = self.entries[index].resource.clone();
        let mut scope = incoming.to_vec();
        if !scope.contains(&resource) {
            scope.push(resource.clone());
        }
        let key = (index, scope.clone());
        if let Some(instance) = self.instances.get(&key) {
            return Ok(format!("#/$defs/n{instance}"));
        }
        if self.instances.len() >= MAX_INSTANTIATIONS {
            return Err(AstError::AnalysisLimit {
                feature: "dynamic reference expansion",
            });
        }
        let instance = self.instances.len();
        self.instances.insert(key, instance);
        let value = self.entries[index].value;
        let mut result = value.clone();
        if let Some(object) = result.as_object_mut() {
            for keyword in ["$id", "$anchor", "$dynamicAnchor", "$defs", "definitions"] {
                object.remove(keyword);
            }
            // The two reference applicators are independent sibling assertions.
            let mut references = Vec::new();
            for keyword in ["$ref", "$dynamicRef"] {
                if let Some(reference) = object
                    .remove(keyword)
                    .and_then(|v| v.as_str().map(str::to_owned))
                {
                    let mut target = self.lookup(&resource, &reference)?;
                    if keyword == "$dynamicRef"
                        && let Some(anchor) = self.entries[target]
                            .value
                            .get("$dynamicAnchor")
                            .and_then(Value::as_str)
                    {
                        let fragment = Url::parse(&resolve(&resource, &reference)?)
                            .expect("resolved URI")
                            .fragment()
                            .map(str::to_owned);
                        if fragment.as_deref() == Some(anchor) {
                            for uri in &scope {
                                if let Some(candidate) =
                                    self.anchors.get(&resolve(uri, &format!("#{anchor}"))?)
                                    && self.entries[*candidate]
                                        .value
                                        .get("$dynamicAnchor")
                                        .and_then(Value::as_str)
                                        == Some(anchor)
                                {
                                    target = *candidate;
                                    break;
                                }
                            }
                        }
                    }
                    references.push(json!({"$ref": self.instantiate(target, &scope)?}));
                }
            }
            for (suffix, _) in children(value) {
                let first = suffix.split('/').next().expect("child keyword");
                if matches!(first, "$defs" | "definitions" | "contentSchema") {
                    continue;
                }
                let pointer = format!("{}/{suffix}", self.entries[index].pointer);
                let child = *self.pointers.get(&pointer).expect("indexed child");
                let replacement = json!({"$ref":self.instantiate(child, &scope)?});
                // Assignment uses the same escaped JSON Pointer emitted by indexing.
                set_child(object, &suffix, replacement);
            }
            if !references.is_empty() {
                if references.len() == 1 {
                    object.insert("$ref".into(), references.remove(0)["$ref"].clone());
                } else {
                    let existing = object.entry("allOf").or_insert_with(|| json!([]));
                    existing
                        .as_array_mut()
                        .expect("validated allOf")
                        .extend(references);
                }
            }
        }
        self.definitions.insert(format!("n{instance}"), result);
        Ok(format!("#/$defs/n{instance}"))
    }
}

fn set_child(object: &mut Map<String, Value>, suffix: &str, value: Value) {
    let (keyword, rest) = suffix.split_once('/').unwrap_or((suffix, ""));
    if rest.is_empty() {
        object.insert(keyword.into(), value);
    } else {
        *object
            .get_mut(keyword)
            .expect("keyword")
            .pointer_mut(&format!("/{rest}"))
            .expect("child") = value;
    }
}

pub(crate) fn children(value: &Value) -> Vec<(String, &Value)> {
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for keyword in SINGLE_SCHEMA_CHILD_KEYWORDS {
        if let Some(child) = object.get(keyword) {
            result.push((keyword.to_owned(), child));
        }
    }
    for keyword in SCHEMA_MAP_CHILD_KEYWORDS.into_iter().chain(["definitions"]) {
        if let Some(children) = object.get(keyword).and_then(Value::as_object) {
            result.extend(children.iter().map(|(key, value)| {
                (
                    format!("{keyword}/{}", key.replace('~', "~0").replace('/', "~1")),
                    value,
                )
            }));
        }
    }
    if let Some(dependencies) = object.get("dependencies").and_then(Value::as_object) {
        result.extend(
            dependencies
                .iter()
                .filter(|(_, value)| !value.is_array())
                .map(|(key, value)| {
                    (
                        format!("dependencies/{}", key.replace('~', "~0").replace('/', "~1")),
                        value,
                    )
                }),
        );
    }
    for keyword in SCHEMA_ARRAY_CHILD_KEYWORDS {
        if let Some(children) = object.get(keyword).and_then(Value::as_array) {
            result.extend(
                children
                    .iter()
                    .enumerate()
                    .map(|(i, value)| (format!("{keyword}/{i}"), value)),
            );
        }
    }
    result
}
