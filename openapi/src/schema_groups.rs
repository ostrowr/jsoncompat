//! Partition component models without splitting reference-connected schemas.
use super::{
    ComponentSchemaDef, JsonPointer, OpenApiDocument, OpenApiError, invalid_value,
    load_component_schema_defs_for_validation,
};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroUsize,
};

/// Components that must be generated together to preserve shared class identity
/// and recursion. Independent components may share a bounded module.
pub struct ComponentSchemaGroup {
    pub schemas: Map<String, Value>,
    pub dialect: String,
}
impl OpenApiDocument {
    pub fn component_schema_groups(
        &self,
        selected: &BTreeSet<String>,
        models_per_module: NonZeroUsize,
    ) -> Result<Vec<ComponentSchemaGroup>, OpenApiError> {
        self.component_schema_groups_with_schemas(selected, models_per_module, &Map::new())
    }

    /// Include synthetic operation schemas in the same reference graph as the
    /// components. Extra schemas must already use lowered `#/$defs/` references.
    pub fn component_schema_groups_with_schemas(
        &self,
        selected: &BTreeSet<String>,
        models_per_module: NonZeroUsize,
        extra: &Map<String, Value>,
    ) -> Result<Vec<ComponentSchemaGroup>, OpenApiError> {
        let mut available = load_component_schema_defs_for_validation(self)?;
        for (name, schema) in extra {
            if available.contains_key(name) {
                return Err(invalid_value(
                    &JsonPointer::root().child(name),
                    "a unique generated schema name",
                ));
            }
            available.insert(
                name.clone(),
                ComponentSchemaDef {
                    schema: schema.clone(),
                    dependencies: super::collect_component_schema_names(schema),
                },
            );
        }
        let dialect = self.supported_schema_dialect()?.uri().to_owned();
        for name in selected {
            if !available.contains_key(name) {
                return Err(invalid_value(
                    &JsonPointer::root()
                        .child("components")
                        .child("schemas")
                        .child(name),
                    "an existing component selected for generation",
                ));
            }
        }
        // URI/anchor references can cross component boundaries without a local
        // pointer. Keep their resource scope intact rather than guess a graph.
        let scoped = available
            .values()
            .any(|entry| has_resource_scope(&entry.schema));
        let mut pending: Vec<_> = if selected.is_empty() || scoped {
            available.keys().cloned().collect()
        } else {
            selected.iter().cloned().collect()
        };
        pending.extend(extra.keys().cloned());
        let mut included = BTreeSet::new();
        while let Some(name) = pending.pop() {
            if !included.insert(name.clone()) {
                continue;
            }
            let entry = available.get(&name).ok_or_else(|| {
                invalid_value(
                    &JsonPointer::root()
                        .child("components")
                        .child("schemas")
                        .child(&name),
                    "a referenced component schema",
                )
            })?;
            pending.extend(entry.dependencies.iter().cloned());
        }
        let mut neighbors: BTreeMap<String, Vec<String>> = included
            .iter()
            .map(|name| (name.clone(), Vec::new()))
            .collect();
        for name in &included {
            for dependency in &available[name].dependencies {
                neighbors.get_mut(name).unwrap().push(dependency.clone());
                neighbors.get_mut(dependency).unwrap().push(name.clone());
            }
        }
        if scoped {
            return Ok(vec![ComponentSchemaGroup {
                schemas: collect(&mut available, included),
                dialect,
            }]);
        }
        let mut groups = Vec::new();
        let mut current = BTreeSet::new();
        while let Some(first) = included.pop_first() {
            let mut connected = BTreeSet::from([first.clone()]);
            let mut pending = vec![first];
            while let Some(name) = pending.pop() {
                for neighbor in &neighbors[&name] {
                    if included.remove(neighbor) {
                        connected.insert(neighbor.clone());
                        pending.push(neighbor.clone());
                    }
                }
            }
            if !current.is_empty() && current.len() + connected.len() > models_per_module.get() {
                groups.push(ComponentSchemaGroup {
                    schemas: collect(&mut available, std::mem::take(&mut current)),
                    dialect: dialect.clone(),
                });
            }
            current.extend(connected);
        }
        if !current.is_empty() {
            groups.push(ComponentSchemaGroup {
                schemas: collect(&mut available, current),
                dialect,
            });
        }
        Ok(groups)
    }
}
fn collect(
    available: &mut BTreeMap<String, ComponentSchemaDef>,
    names: BTreeSet<String>,
) -> Map<String, Value> {
    names
        .into_iter()
        .map(|name| {
            let schema = available
                .remove(&name)
                .expect("schema groups partition the selected component names")
                .schema;
            (name, schema)
        })
        .collect()
}
fn has_resource_scope(schema: &Value) -> bool {
    if schema.get("$id").is_some()
        || schema.get("$anchor").is_some()
        || schema.get("$dynamicAnchor").is_some()
    {
        return true;
    }
    if ["$ref", "$dynamicRef"].iter().any(|key| {
        schema
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|reference| !reference.starts_with("#/$defs/"))
    }) {
        return true;
    }
    for key in json_schema_ast::SINGLE_SCHEMA_CHILD_KEYWORDS {
        if schema.get(key).is_some_and(has_resource_scope) {
            return true;
        }
    }
    for key in json_schema_ast::SCHEMA_MAP_CHILD_KEYWORDS {
        if schema
            .get(key)
            .and_then(Value::as_object)
            .is_some_and(|map| map.values().any(has_resource_scope))
        {
            return true;
        }
    }
    for key in json_schema_ast::SCHEMA_ARRAY_CHILD_KEYWORDS {
        if schema
            .get(key)
            .and_then(Value::as_array)
            .is_some_and(|array| array.iter().any(has_resource_scope))
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OpenApiOperationLowerer;
    use serde_json::json;

    #[test]
    fn groups_partition_and_preserve_the_prepared_component_schemas() {
        let document = OpenApiDocument::from_json(&json!({
            "openapi":"3.1.0", "info":{"title":"Grouped components", "version":"1"},
            "paths":{},
            "components":{"schemas":{
                "A":{"type":"object", "properties":{
                    "count":{"type":"integer", "minimum":0},
                    "next":{"$ref":"#/components/schemas/B"}
                }, "required":["count"]},
                "B":{"type":"object", "properties":{
                    "owner":{"$ref":"#/components/schemas/A"},
                    "literal":{"const":{"$ref":"this is instance data"}}
                }},
                "C":false,
                "D":{"type":"array", "items":{"$ref":"#/components/schemas/A/properties/count"}}
            }}
        }))
        .unwrap();
        let lowerer = OpenApiOperationLowerer::new(&document).unwrap();
        assert_eq!(lowerer.component_schemas().len(), 4);
        let definitions: Map<_, _> = lowerer
            .component_schemas()
            .map(|(name, schema)| (name.to_owned(), schema.clone()))
            .collect();
        let size = NonZeroUsize::new(1).unwrap();
        let groups = document
            .component_schema_groups(&BTreeSet::new(), size)
            .unwrap();
        assert_eq!(groups.len(), 2);
        let flattened: Map<_, _> = groups.into_iter().flat_map(|group| group.schemas).collect();
        assert_eq!(flattened, definitions);
        assert_eq!(definitions["C"], false);
        assert_eq!(
            definitions["B"]["properties"]["literal"]["const"]["$ref"],
            "this is instance data"
        );
        assert_eq!(
            definitions["D"]["items"]["$ref"],
            "#/$defs/A/properties/count"
        );

        let selected = document
            .component_schema_groups(&BTreeSet::from(["D".into()]), size)
            .unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0]
                .schemas
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["A", "B", "D"]
        );
        assert!(
            selected[0]
                .schemas
                .iter()
                .all(|(name, schema)| definitions.get(name) == Some(schema))
        );
    }
}
