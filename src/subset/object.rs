//! Object subset helpers.

use crate::SchemaNode;
use crate::subset::{SubschemaCheckContext, is_subschema_of_with_productive_context};
use json_schema_ast::{
    CountRange, PatternConstraint, PatternProperty, PatternSupport, SchemaNodeKind,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::{scalar::check_enum_inclusion, schema_may_under_accept_values};

pub(super) struct ObjectConstraints<'a> {
    pub(super) properties: &'a HashMap<String, SchemaNode>,
    pub(super) pattern_properties: &'a HashMap<String, PatternProperty<SchemaNode>>,
    pub(super) required: &'a HashSet<String>,
    pub(super) additional: &'a SchemaNode,
    pub(super) property_names: &'a SchemaNode,
    pub(super) property_count: CountRange<usize>,
    pub(super) dependent_required: &'a HashMap<String, Vec<String>>,
    pub(super) enumeration: Option<&'a [Value]>,
}

#[derive(Clone, Copy)]
struct SubPropertyConjuncts<'a> {
    property: Option<&'a SchemaNode>,
    pattern_properties: &'a HashMap<String, PatternProperty<SchemaNode>>,
    additional: &'a SchemaNode,
}

pub(super) fn object_constraints_subsumed(
    sub: ObjectConstraints<'_>,
    sup: ObjectConstraints<'_>,
    context: &mut SubschemaCheckContext,
) -> bool {
    if !sup.property_count.contains_range(sub.property_count)
        || !check_enum_inclusion(sub.enumeration, sup.enumeration)
        || !sup.required.is_subset(sub.required)
    {
        return false;
    }

    for (property_name, sub_schema) in sub.properties {
        if !object_property_schema_is_subsumed(
            property_name,
            SubPropertyConjuncts {
                property: Some(sub_schema),
                pattern_properties: sub.pattern_properties,
                additional: sub.additional,
            },
            sup.properties.get(property_name),
            sup.pattern_properties,
            sup.additional,
            context,
        ) {
            return false;
        }
    }

    for (property_name, sup_property_schema) in sup.properties {
        if sub.properties.contains_key(property_name)
            || subset_property_conjuncts_subsume_schema(
                property_name,
                SubPropertyConjuncts {
                    property: None,
                    pattern_properties: sub.pattern_properties,
                    additional: sub.additional,
                },
                sup_property_schema,
                context,
            )
        {
            continue;
        }

        return false;
    }

    for (pattern, sub_pattern_property) in sub.pattern_properties {
        let sup_schema = match sup.pattern_properties.get(pattern) {
            Some(sup_pattern_property) => &sup_pattern_property.schema,
            None if sup.pattern_properties.is_empty() => sup.additional,
            None => return false,
        };
        if !is_subschema_of_with_productive_context(
            &sub_pattern_property.schema,
            sup_schema,
            context,
        ) {
            return false;
        }

        // A property name matched by one pattern can also match any other
        // pattern. Without proving regex disjointness, preserve every
        // additional superset pattern constraint conservatively.
        for (sup_pattern, sup_pattern_property) in sup.pattern_properties {
            if sup_pattern == pattern {
                continue;
            }
            if !is_subschema_of_with_productive_context(
                &sub_pattern_property.schema,
                &sup_pattern_property.schema,
                context,
            ) {
                return false;
            }
        }
    }

    if !object_additional_schema_is_subsumed(
        sub.additional,
        sub.pattern_properties,
        sup.pattern_properties,
        sup.additional,
        context,
    ) || !is_subschema_of_with_productive_context(
        sub.property_names,
        sup.property_names,
        context,
    ) {
        return false;
    }

    sup.dependent_required
        .iter()
        .all(|(trigger, dependencies)| {
            !object_property_name_can_be_present(
                trigger,
                sub.properties,
                sub.pattern_properties,
                sub.property_names,
                sub.additional,
                context,
            ) || dependencies.iter().all(|dependency| {
                dependent_requirement_is_guaranteed(
                    trigger,
                    dependency,
                    sub.required,
                    sub.dependent_required,
                )
            })
        })
}

pub(super) fn dependent_requirement_is_guaranteed(
    trigger: &str,
    dependency: &str,
    required: &HashSet<String>,
    dependent_required: &HashMap<String, Vec<String>>,
) -> bool {
    if dependency == trigger || required.contains(dependency) {
        return true;
    }

    let mut pending = vec![trigger];
    let mut visited = HashSet::new();
    while let Some(current) = pending.pop() {
        if !visited.insert(current) {
            continue;
        }
        let Some(dependencies) = dependent_required.get(current) else {
            continue;
        };
        for guaranteed_dependency in dependencies {
            if guaranteed_dependency == dependency {
                return true;
            }
            pending.push(guaranteed_dependency);
        }
    }

    false
}

fn object_property_schema_is_subsumed(
    property_name: &str,
    sub: SubPropertyConjuncts<'_>,
    sup_property_schema: Option<&SchemaNode>,
    sup_pattern_properties: &HashMap<String, PatternProperty<SchemaNode>>,
    sup_additional: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let mut matched = false;

    if let Some(sup_property_schema) = sup_property_schema {
        matched = true;
        if !subset_property_conjuncts_subsume_schema(
            property_name,
            sub,
            sup_property_schema,
            context,
        ) {
            return false;
        }
    }

    for sup_pattern_property in sup_pattern_properties.values() {
        if !pattern_may_match_property_name(&sup_pattern_property.pattern, property_name) {
            continue;
        }
        matched = true;
        if !subset_property_conjuncts_subsume_schema(
            property_name,
            sub,
            &sup_pattern_property.schema,
            context,
        ) {
            return false;
        }
    }

    matched || subset_property_conjuncts_subsume_schema(property_name, sub, sup_additional, context)
}

fn subset_property_conjuncts_subsume_schema(
    property_name: &str,
    sub: SubPropertyConjuncts<'_>,
    sup_schema: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if let Some(sub_property_schema) = sub.property
        && is_subschema_of_with_productive_context(sub_property_schema, sup_schema, context)
    {
        return true;
    }

    let mut has_maybe_matching_pattern = false;
    for sub_pattern_property in sub.pattern_properties.values() {
        if !pattern_may_match_property_name(&sub_pattern_property.pattern, property_name) {
            continue;
        }

        has_maybe_matching_pattern = true;
        if pattern_definitely_matches_property_name(&sub_pattern_property.pattern, property_name)
            && is_subschema_of_with_productive_context(
                &sub_pattern_property.schema,
                sup_schema,
                context,
            )
        {
            return true;
        }
    }

    sub.property.is_none()
        && !has_maybe_matching_pattern
        && (context.assume_subset_omits_undeclared_properties
            || is_subschema_of_with_productive_context(sub.additional, sup_schema, context))
}

pub(super) fn implicit_property_conjuncts_subsume_schema(
    property_name: &str,
    pattern_properties: &HashMap<String, PatternProperty<SchemaNode>>,
    additional: &SchemaNode,
    sup_schema: &SchemaNode,
) -> bool {
    subset_property_conjuncts_subsume_schema(
        property_name,
        SubPropertyConjuncts {
            property: None,
            pattern_properties,
            additional,
        },
        sup_schema,
        &mut SubschemaCheckContext::default(),
    )
}

fn object_property_name_can_be_present(
    property_name: &str,
    properties: &HashMap<String, SchemaNode>,
    pattern_properties: &HashMap<String, PatternProperty<SchemaNode>>,
    property_names: &SchemaNode,
    additional: &SchemaNode,
    context: &SubschemaCheckContext,
) -> bool {
    if !property_name_schema_may_accept(property_names, property_name) {
        return false;
    }

    let explicit_property_can_admit = if let Some(schema) = properties.get(property_name) {
        if matches!(schema.kind(), SchemaNodeKind::BoolSchema(false)) {
            return false;
        }
        true
    } else {
        false
    };

    let mut definite_pattern_can_admit = false;
    for pattern_property in pattern_properties.values() {
        if !pattern_definitely_matches_property_name(&pattern_property.pattern, property_name) {
            continue;
        }
        if matches!(
            pattern_property.schema.kind(),
            SchemaNodeKind::BoolSchema(false)
        ) {
            return false;
        }
        definite_pattern_can_admit = true;
    }

    if explicit_property_can_admit || definite_pattern_can_admit {
        return true;
    }

    if !context.assume_subset_omits_undeclared_properties
        && !matches!(additional.kind(), SchemaNodeKind::BoolSchema(false))
    {
        return true;
    }

    pattern_properties.values().any(|pattern_property| {
        pattern_may_match_property_name(&pattern_property.pattern, property_name)
            && !matches!(
                pattern_property.schema.kind(),
                SchemaNodeKind::BoolSchema(false)
            )
    })
}

fn property_name_schema_may_accept(schema: &SchemaNode, property_name: &str) -> bool {
    let candidate = Value::String(property_name.to_owned());
    schema.accepts_value(&candidate) || schema_may_under_accept_values(schema)
}

fn object_additional_schema_is_subsumed(
    sub_additional: &SchemaNode,
    sub_pattern_properties: &HashMap<String, PatternProperty<SchemaNode>>,
    sup_pattern_properties: &HashMap<String, PatternProperty<SchemaNode>>,
    sup_additional: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if context.assume_subset_omits_undeclared_properties {
        return true;
    }

    if !is_subschema_of_with_productive_context(sub_additional, sup_additional, context) {
        return false;
    }

    sup_pattern_properties
        .iter()
        .filter(|(pattern, _)| !sub_pattern_properties.contains_key(*pattern))
        .all(|(_, sup_pattern_property)| {
            is_subschema_of_with_productive_context(
                sub_additional,
                &sup_pattern_property.schema,
                context,
            )
        })
}

fn pattern_definitely_matches_property_name(
    pattern: &PatternConstraint,
    property_name: &str,
) -> bool {
    match pattern.support() {
        PatternSupport::Supported => pattern.is_match(property_name),
        PatternSupport::Unsupported => false,
    }
}

fn pattern_may_match_property_name(pattern: &PatternConstraint, property_name: &str) -> bool {
    match pattern.support() {
        PatternSupport::Supported => pattern.is_match(property_name),
        PatternSupport::Unsupported => true,
    }
}
