//! Object subset helpers.

use crate::SchemaNode;
use crate::subset::{SubschemaCheckContext, is_subschema_of_with_productive_context};
use json_schema_ast::{CountRange, PatternConstraint, PatternProperty, PatternSupport};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::{
    finite_schema_value_superset, scalar::check_enum_inclusion,
    schema_definitely_rejects_all_values, schema_may_under_accept_values,
};

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

/// Return whether `property_name` can coexist with the subset's required names
/// under its maxProperties bound.
pub(super) fn property_name_can_fit_count(
    property_name: &str,
    required: &HashSet<String>,
    property_count: CountRange<usize>,
) -> bool {
    required.contains(property_name)
        || property_count
            .max()
            .is_none_or(|max_properties| required.len() < max_properties)
}

/// Like [`property_name_can_fit_count`], but also accounts for names that
/// would be forced by `dependentRequired` if `property_name` were present.
///
/// `required` is expected to already include the closure of unconditionally
/// required names. Adding an optional trigger can still force a fresh chain of
/// dependencies; if that chain overflows `maxProperties`, the trigger is
/// impossible and constraints guarded by it are vacuous.
pub(super) fn property_name_can_fit_with_dependencies(
    property_name: &str,
    required: &HashSet<String>,
    property_count: CountRange<usize>,
    dependent_required: &HashMap<String, Vec<String>>,
) -> bool {
    if !property_name_can_fit_count(property_name, required, property_count) {
        return false;
    }

    let Some(max_properties) = property_count.max() else {
        return true;
    };

    if required.len() > max_properties {
        return false;
    }
    if required.contains(property_name) {
        return true;
    }

    names_forced_by_required_and_property(required, property_name, dependent_required).len()
        <= max_properties
}

/// Check `propertyNames` implication, with a small cardinality shortcut.
///
/// When `maxProperties` is exactly filled by names guaranteed by the subset
/// (including names forced through `dependentRequired`), no other property name
/// can occur. In that case it is enough to check those literal names against
/// the superset's `propertyNames` schema instead
/// of requiring a global schema implication (which is often much stronger than
/// necessary). If the guaranteed set already exceeds `maxProperties`, the subset
/// object type is uninhabited, so this particular constraint is vacuous.
pub(super) fn property_names_subsumed_with_count(
    sub_property_names: &SchemaNode,
    sup_property_names: &SchemaNode,
    required: &HashSet<String>,
    property_count: CountRange<usize>,
    context: &mut SubschemaCheckContext,
) -> bool {
    if let Some(max_properties) = property_count.max()
        && required.len() >= max_properties
    {
        if required.len() > max_properties {
            return true;
        }

        for name in required {
            // A required name rejected by the subset's own propertyNames makes
            // the object branch uninhabited. `property_name_schema_may_accept`
            // only returns false when that rejection is definitive.
            if !property_name_schema_may_accept(sub_property_names, name) {
                return true;
            }
            if !context.superset_contains_value(sup_property_names, &Value::String(name.clone())) {
                return false;
            }
        }
        return true;
    }

    is_subschema_of_with_productive_context(sub_property_names, sup_property_names, context)
}

fn dependent_required_constraints_subsumed(
    sub: &ObjectConstraints<'_>,
    sup: &ObjectConstraints<'_>,
    guaranteed_names: &HashSet<String>,
    context: &mut SubschemaCheckContext,
) -> bool {
    sup.dependent_required
        .iter()
        .all(|(trigger, dependencies)| {
            !object_property_name_can_be_present(trigger, sub, guaranteed_names, context)
                || dependencies.iter().all(|dependency| {
                    dependent_requirement_is_guaranteed(
                        trigger,
                        dependency,
                        guaranteed_names,
                        sub.dependent_required,
                    )
                })
        })
}

pub(super) fn effective_property_count_with_forced_names(
    property_count: CountRange<usize>,
    forced_names: &HashSet<String>,
) -> Option<CountRange<usize>> {
    CountRange::new(
        property_count.min().max(forced_names.len()),
        property_count.max(),
    )
}

pub(super) fn object_constraints_subsumed(
    sub: ObjectConstraints<'_>,
    sup: ObjectConstraints<'_>,
    context: &mut SubschemaCheckContext,
) -> bool {
    // If the subset object branch has no inhabitants, it is trivially a
    // subset of every superset branch.  Keep this check before the required
    // and count implication checks below: those checks otherwise report
    // false negatives for schemas such as
    // `{type: object, required: ["x"], properties: {x: false}}`.
    if object_constraints_definitely_uninhabited(&sub) {
        return true;
    }

    let guaranteed_names_storage = (!sub.dependent_required.is_empty())
        .then(|| names_forced_by_required(sub.required, sub.dependent_required));
    let guaranteed_names = guaranteed_names_storage.as_ref().unwrap_or(sub.required);
    let Some(mut effective_sub_property_count) =
        effective_property_count_with_forced_names(sub.property_count, guaranteed_names)
    else {
        return true;
    };
    if let Some(name_capacity) = finite_object_name_capacity(&sub) {
        let capped_max = Some(
            effective_sub_property_count
                .max()
                .map_or(name_capacity, |max| max.min(name_capacity)),
        );
        let Some(capped_count) = CountRange::new(effective_sub_property_count.min(), capped_max)
        else {
            return true;
        };
        effective_sub_property_count = capped_count;
    }

    if !sup
        .property_count
        .contains_range(effective_sub_property_count)
        || !check_enum_inclusion(sub.enumeration, sup.enumeration)
        || !sup.required.is_subset(guaranteed_names)
    {
        return false;
    }

    // Once the subset is known to admit at most the empty object, all
    // per-property constraints in the superset are vacuous.  The range and
    // required checks above already ruled out supersets that reject `{}`.
    // Avoid walking property schemas/patterns here: doing so is both wasted
    // work and can be spuriously conservative for names that cannot exist.
    if effective_sub_property_count.max() == Some(0) {
        return true;
    }

    // When propertyNames has an exact finite language, there are no truly
    // arbitrary property names. Enumerate the possible names directly instead
    // of requiring global additionalProperties/patternProperties implication,
    // which is often much stronger than necessary.
    if let Some(finite_names) = finite_object_name_values(&sub) {
        for property_name in &finite_names {
            if !object_property_name_can_be_present(property_name, &sub, guaranteed_names, context)
            {
                continue;
            }
            if !context
                .superset_contains_value(sup.property_names, &Value::String(property_name.clone()))
            {
                return false;
            }
            if !object_property_schema_is_subsumed(
                property_name,
                SubPropertyConjuncts {
                    property: sub.properties.get(property_name),
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
        return dependent_required_constraints_subsumed(&sub, &sup, guaranteed_names, context);
    }

    for (property_name, sub_schema) in sub.properties {
        if !property_name_can_fit_with_dependencies(
            property_name,
            guaranteed_names,
            effective_sub_property_count,
            sub.dependent_required,
        ) {
            continue;
        }
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
        if !property_name_can_fit_with_dependencies(
            property_name,
            guaranteed_names,
            effective_sub_property_count,
            sub.dependent_required,
        ) {
            continue;
        }
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

    if let Some(max_properties) = effective_sub_property_count.max()
        && guaranteed_names.len() >= max_properties
    {
        // If guaranteed names exhaust the property budget, there are no
        // arbitrary additional names to reason about. Check the finite set of
        // guaranteed names directly against the superset's property/pattern/
        // additional constraints, then skip the global pattern/additional
        // implication checks that assume arbitrary names may occur.
        if guaranteed_names.len() > max_properties {
            return true;
        }
        for property_name in guaranteed_names {
            if !object_property_schema_is_subsumed(
                property_name,
                SubPropertyConjuncts {
                    property: sub.properties.get(property_name),
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
        if !property_names_subsumed_with_count(
            sub.property_names,
            sup.property_names,
            guaranteed_names,
            effective_sub_property_count,
            context,
        ) {
            return false;
        }
        return dependent_required_constraints_subsumed(&sub, &sup, guaranteed_names, context);
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
    ) || !property_names_subsumed_with_count(
        sub.property_names,
        sup.property_names,
        guaranteed_names,
        effective_sub_property_count,
        context,
    ) {
        return false;
    }

    dependent_required_constraints_subsumed(&sub, &sup, guaranteed_names, context)
}

/// A deliberately small, sound emptiness check for object constraints.
///
/// Full JSON Schema satisfiability is hard, but a few contradictions are
/// common in generated schemas and cheap to recognize: too many required
/// names for `maxProperties`, a required name rejected by `propertyNames`, or
/// a required property whose applicable schema is literally `false`.
fn object_constraints_definitely_uninhabited(sub: &ObjectConstraints<'_>) -> bool {
    // `dependentRequired` edges rooted at an unconditionally required name are
    // themselves unconditional. Close over those edges before applying the
    // cheap cardinality/property-name contradiction checks below.
    let forced_names_storage = (!sub.dependent_required.is_empty())
        .then(|| names_forced_by_required(sub.required, sub.dependent_required));
    let forced_names = forced_names_storage.as_ref().unwrap_or(sub.required);

    if sub
        .property_count
        .max()
        .is_some_and(|max_properties| forced_names.len() > max_properties)
    {
        return true;
    }

    if forced_names_have_definite_contradiction(
        forced_names,
        sub.properties,
        sub.pattern_properties,
        sub.property_names,
        sub.additional,
    ) {
        return true;
    }

    // A finite propertyNames language also caps the number of distinct object
    // keys (JSON objects cannot contain the same key twice).  This catches
    // contradictions such as `propertyNames: { enum: ["a"] }` together with
    // `minProperties: 2`, even when additionalProperties is otherwise open.
    if let Some(name_capacity) = finite_property_name_capacity(sub.property_names)
        && name_capacity < sub.property_count.min()
    {
        return true;
    }

    // If there are no pattern properties and additionalProperties is exactly
    // false, only explicitly declared property names can ever appear.  This
    // gives a cheap finite capacity bound for minProperties.  Count only names
    // that are not definitely ruled out by their own schema or propertyNames;
    // over-counting is fine (it merely misses an emptiness shortcut), but
    // under-counting would be unsound.
    if sub.pattern_properties.is_empty()
        && schema_definitely_rejects_all_values(sub.additional)
        && sub.property_count.min() > 0
    {
        let possible_declared_names = sub
            .properties
            .iter()
            .filter(|(name, schema)| {
                if schema_definitely_rejects_all_values(schema)
                    || !property_name_schema_may_accept(sub.property_names, name)
                    || !property_name_can_fit_with_dependencies(
                        name,
                        forced_names,
                        sub.property_count,
                        sub.dependent_required,
                    )
                {
                    return false;
                }

                let forced_if_present = names_forced_by_required_and_property(
                    forced_names,
                    name,
                    sub.dependent_required,
                );
                !forced_names_have_definite_contradiction(
                    &forced_if_present,
                    sub.properties,
                    sub.pattern_properties,
                    sub.property_names,
                    sub.additional,
                )
            })
            .count();
        if possible_declared_names < sub.property_count.min() {
            return true;
        }
    }

    false
}

/// Best-effort finite upper bound for the set of property names accepted by a
/// propertyNames schema. Return `None` unless the bound is exact enough to be
/// sound for raw validation too.
pub(super) fn finite_property_name_capacity(schema: &SchemaNode) -> Option<usize> {
    finite_property_name_values(schema).map(|names| names.len())
}

fn finite_object_name_capacity(sub: &ObjectConstraints<'_>) -> Option<usize> {
    let mut capacity = finite_property_name_capacity(sub.property_names);
    if sub.pattern_properties.is_empty() && schema_definitely_rejects_all_values(sub.additional) {
        let declared = sub.properties.len();
        capacity = Some(capacity.map_or(declared, |cap| cap.min(declared)));
    }
    capacity
}

/// A finite superset of all names that can occur in this object branch.
fn finite_object_name_values(sub: &ObjectConstraints<'_>) -> Option<Vec<String>> {
    let property_names_values = finite_property_name_values(sub.property_names);
    let closed_declared_values = (sub.pattern_properties.is_empty()
        && schema_definitely_rejects_all_values(sub.additional))
    .then(|| sub.properties.keys().cloned().collect::<Vec<_>>());

    match (property_names_values, closed_declared_values) {
        (Some(names), Some(declared)) => {
            // Both constraints are exact finite supersets; their intersection
            // is still a sound (and tighter) finite superset of possible names.
            Some(
                names
                    .into_iter()
                    .filter(|name| declared.iter().any(|declared| declared == name))
                    .collect(),
            )
        }
        (Some(names), None) | (None, Some(names)) => Some(names),
        (None, None) => None,
    }
}

/// Exact finite set of strings accepted by a simple propertyNames schema.
/// Returns `None` whenever the internal evaluator may fail closed/open relative
/// to raw validation, or when the language is not obviously finite.
fn finite_property_name_values(schema: &SchemaNode) -> Option<Vec<String>> {
    let values = finite_schema_value_superset(schema)?;
    let mut strings: Vec<String> = Vec::new();
    for value in values {
        let Some(s) = value.as_str() else {
            // Non-string JSON values cannot be object property names.
            continue;
        };
        if !strings.iter().any(|seen| seen == s) {
            strings.push(s.to_owned());
        }
    }
    Some(strings)
}

/// Return true only for contradictions that definitively make all objects with
/// every name in `forced_names` impossible. Unsupported regex/propertyNames
/// cases deliberately stay "maybe" so this remains a sound shortcut.
fn forced_names_have_definite_contradiction(
    forced_names: &HashSet<String>,
    properties: &HashMap<String, SchemaNode>,
    pattern_properties: &HashMap<String, PatternProperty<SchemaNode>>,
    property_names: &SchemaNode,
    additional: &SchemaNode,
) -> bool {
    for property_name in forced_names {
        if !property_name_schema_may_accept(property_names, property_name) {
            return true;
        }

        if properties
            .get(property_name)
            .is_some_and(schema_definitely_rejects_all_values)
        {
            return true;
        }

        let mut maybe_matched_by_pattern = false;
        for pattern_property in pattern_properties.values() {
            if !pattern_may_match_property_name(&pattern_property.pattern, property_name) {
                continue;
            }
            maybe_matched_by_pattern = true;
            if pattern_definitely_matches_property_name(&pattern_property.pattern, property_name)
                && schema_definitely_rejects_all_values(&pattern_property.schema)
            {
                return true;
            }
        }

        // An undeclared forced property that cannot match any pattern must be
        // accepted by additionalProperties. If that schema is literally false,
        // the object branch is empty.
        if !properties.contains_key(property_name)
            && !maybe_matched_by_pattern
            && schema_definitely_rejects_all_values(additional)
        {
            return true;
        }
    }

    false
}

fn names_forced_by_required_and_property(
    required: &HashSet<String>,
    property_name: &str,
    dependent_required: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut seeds = required.clone();
    seeds.insert(property_name.to_owned());
    names_forced_by_required(&seeds, dependent_required)
}

pub(super) fn names_forced_by_required(
    required: &HashSet<String>,
    dependent_required: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut forced = required.clone();
    let mut pending = required.iter().cloned().collect::<Vec<_>>();

    while let Some(name) = pending.pop() {
        let Some(dependencies) = dependent_required.get(&name) else {
            continue;
        };
        for dependency in dependencies {
            if forced.insert(dependency.clone()) {
                pending.push(dependency.clone());
            }
        }
    }

    forced
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
    sub: &ObjectConstraints<'_>,
    required: &HashSet<String>,
    context: &SubschemaCheckContext,
) -> bool {
    // If all available property slots are already consumed by required names,
    // a distinct optional trigger cannot be present. This is a cheap cardinality
    // check that avoids treating saturated objects as if arbitrary extra names
    // were still possible.
    if !property_name_can_fit_with_dependencies(
        property_name,
        required,
        sub.property_count,
        sub.dependent_required,
    ) {
        return false;
    }

    let forced_if_present =
        names_forced_by_required_and_property(required, property_name, sub.dependent_required);
    if forced_names_have_definite_contradiction(
        &forced_if_present,
        sub.properties,
        sub.pattern_properties,
        sub.property_names,
        sub.additional,
    ) {
        return false;
    }

    if !property_name_schema_may_accept(sub.property_names, property_name) {
        return false;
    }

    let explicit_property_can_admit = if let Some(schema) = sub.properties.get(property_name) {
        if schema_definitely_rejects_all_values(schema) {
            return false;
        }
        true
    } else {
        false
    };

    let mut definite_pattern_can_admit = false;
    for pattern_property in sub.pattern_properties.values() {
        if !pattern_definitely_matches_property_name(&pattern_property.pattern, property_name) {
            continue;
        }
        if schema_definitely_rejects_all_values(&pattern_property.schema) {
            return false;
        }
        definite_pattern_can_admit = true;
    }

    if explicit_property_can_admit || definite_pattern_can_admit {
        return true;
    }

    if !context.assume_subset_omits_undeclared_properties
        && !schema_definitely_rejects_all_values(sub.additional)
    {
        return true;
    }

    sub.pattern_properties.values().any(|pattern_property| {
        pattern_may_match_property_name(&pattern_property.pattern, property_name)
            && !schema_definitely_rejects_all_values(&pattern_property.schema)
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
