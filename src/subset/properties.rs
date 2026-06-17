//! Object property-name/value facts used by disjointness and finite proofs.
//!
//! All predicates are conservative: a negative result means unknown, not overlap.

use super::*;

pub(super) fn guaranteed_property_name_closure(schema: &SchemaNode) -> HashSet<String> {
    fn collect(
        schema: &SchemaNode,
        names: &mut HashSet<String>,
        rules: &mut Vec<(String, Vec<String>)>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::Object {
                required,
                dependent_required,
                ..
            } => {
                names.extend(required.iter().cloned());
                rules.extend(
                    dependent_required
                        .iter()
                        .map(|(trigger, deps)| (trigger.clone(), deps.clone())),
                );
            }
            SchemaNodeKind::Const(value) => {
                if let Some(object) = value.as_object() {
                    names.extend(object.keys().cloned());
                }
            }
            SchemaNodeKind::Enum(values) if !values.is_empty() => {
                let mut iter = values.iter();
                if let Some(first) = iter.next().and_then(Value::as_object) {
                    let mut common: HashSet<String> = first.keys().cloned().collect();
                    for value in iter {
                        if let Some(object) = value.as_object() {
                            common.retain(|name| object.contains_key(name));
                        } else {
                            common.clear();
                            break;
                        }
                    }
                    names.extend(common);
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    collect(child, names, rules, active);
                }
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    let mut names = HashSet::new();
    let mut rules = Vec::new();
    collect(schema, &mut names, &mut rules, &mut HashSet::new());
    let mut changed = true;
    while changed {
        changed = false;
        for (trigger, deps) in &rules {
            if names.contains(trigger) {
                for dep in deps {
                    if names.insert(dep.clone()) {
                        changed = true;
                    }
                }
            }
        }
    }
    names
}

/// Return true when every object admitted by `schema` is known to contain
/// `name`. This is deliberately syntactic and mirrors the discriminator
/// helpers: unions only guarantee names common to all branches, while allOf can
/// inherit a guarantee from any conjunct.
pub(super) fn schema_guarantees_property_name(schema: &SchemaNode, name: &str) -> bool {
    if guaranteed_property_name_closure(schema).contains(name) {
        return true;
    }

    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::Object { required, .. } => {
                required.contains(name) || guaranteed_property_name_closure(schema).contains(name)
            }
            SchemaNodeKind::Const(value) => value
                .as_object()
                .is_some_and(|object| object.contains_key(name)),
            SchemaNodeKind::Enum(values) => {
                !values.is_empty()
                    && values.iter().all(|value| {
                        value
                            .as_object()
                            .is_some_and(|object| object.contains_key(name))
                    })
            }
            SchemaNodeKind::AllOf(children) => {
                guaranteed_property_name_closure(schema).contains(name)
                    || children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty() && children.iter().all(|child| inner(child, name, active))
            }
            SchemaNodeKind::IfThenElse {
                then_schema,
                else_schema,
                ..
            } => {
                match (then_schema.as_ref(), else_schema.as_ref()) {
                    (Some(then_schema), Some(else_schema)) => {
                        inner(then_schema, name, active) && inner(else_schema, name, active)
                    }
                    // A missing branch is unconstrained, so it cannot force a name.
                    _ => false,
                }
            }
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, name, &mut HashSet::new())
}

/// Like `schema_guarantees_property_name`, but scoped to object instances of
/// `schema`.  This is only safe for callers that have already ruled out
/// non-object overlap with the other side; conditional schemas with a missing
/// branch often accept scalars vacuously, so the global guarantee would be too
/// strong there.
pub(super) fn schema_guarantees_property_name_for_objects(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) => false,
            SchemaNodeKind::Object { required, .. } => {
                required.contains(name) || guaranteed_property_name_closure(schema).contains(name)
            }
            SchemaNodeKind::Const(value) => value
                .as_object()
                .is_none_or(|object| object.contains_key(name)),
            SchemaNodeKind::Enum(values) => {
                let mut saw_object = false;
                let mut ok = true;
                for value in values {
                    if let Some(object) = value.as_object() {
                        saw_object = true;
                        if !object.contains_key(name) {
                            ok = false;
                            break;
                        }
                    }
                }
                // No object literals means the object slice is empty.
                !saw_object || ok
            }
            SchemaNodeKind::AllOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut saw_object_branch = false;
                let mut ok = true;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_OBJECT == 0 {
                        continue;
                    }
                    saw_object_branch = true;
                    if !inner(child, name, active) {
                        ok = false;
                        break;
                    }
                }
                !saw_object_branch || ok
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, active) && inner(else_schema, name, active)
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    inner(then_schema, name, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    inner(else_schema, name, active)
                }
                _ => false,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }

    schema_guarantees_property_name(schema, name) || inner(schema, name, &mut HashSet::new())
}

/// Syntactic rejection check for a concrete property name against a
/// `propertyNames` schema. This intentionally recognizes only simple literal,
/// enum, length, and supported-pattern cases; returning false is conservative.
pub(super) fn string_literal_definitely_rejected(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let name_value = Value::String(name.to_owned());
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) | SchemaNodeKind::Any => false,
            SchemaNodeKind::Const(value) => !json_values_equal(value, &name_value),
            SchemaNodeKind::Enum(values) => !values
                .iter()
                .any(|value| json_values_equal(value, &name_value)),
            SchemaNodeKind::String {
                length,
                pattern,
                enumeration,
                ..
            } => {
                if enumeration.as_ref().is_some_and(|values| {
                    !values
                        .iter()
                        .any(|value| json_values_equal(value, &name_value))
                }) {
                    true
                } else {
                    let len = name.chars().count() as u64;
                    len < length.min()
                        || length.max().is_some_and(|max| len > max)
                        || pattern.as_ref().is_some_and(|pattern| {
                            pattern.support() == PatternSupport::Supported
                                && !pattern.is_match(name)
                        })
                }
            }
            SchemaNodeKind::AllOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty() && children.iter().all(|child| inner(child, name, active))
            }
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, name, &mut HashSet::new())
}

/// Return true when `schema` rejects every object containing property `name`.
/// The common generated spelling is `properties: { name: false }`; applicator
/// propagation is included, but we avoid general negation/propertyNames
/// reasoning here to keep the fact obviously sound.
/// Return true when `schema` is known to accept every object containing `name`.
/// This lets `not schema` act as a syntactic "property is absent" guard.  The
/// recognizer is intentionally narrow: object constraints must be universal
/// for arbitrary extra names/values, and applicators are handled only in the
/// directions that preserve a universal-with-property fact.
pub(super) fn schema_accepts_all_objects_with_property(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count,
                dependent_required,
                enumeration,
            } => {
                required.iter().all(|required_name| required_name == name)
                    && properties.keys().all(|property_name| property_name == name)
                    && properties
                        .get(name)
                        .is_none_or(schema_is_trivially_universal)
                    && pattern_properties.is_empty()
                    && property_count.min() <= 1
                    && property_count.max().is_none()
                    && dependent_required.is_empty()
                    && enumeration.is_none()
                    && schema_is_trivially_universal(additional)
                    && schema_is_trivially_universal(property_names)
            }
            // A union accepts all objects-with-name if any branch does.
            SchemaNodeKind::AnyOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            // An intersection does so only if every conjunct does.
            SchemaNodeKind::AllOf(children) => {
                !children.is_empty() && children.iter().all(|child| inner(child, name, active))
            }
            SchemaNodeKind::IfThenElse {
                then_schema,
                else_schema,
                ..
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, active) && inner(else_schema, name, active)
                }
                // The missing branch is implicit `true`; if the present branch
                // accepts every object containing the property, the whole
                // conditional does too regardless of the guard outcome.
                (Some(branch), None) | (None, Some(branch)) => inner(branch, name, active),
                (None, None) => true,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, name, &mut HashSet::new())
}

pub(super) fn schema_forbids_property_name(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::Const(value) => !value
                .as_object()
                .is_some_and(|object| object.contains_key(name)),
            SchemaNodeKind::Enum(values) => values.iter().all(|value| {
                !value
                    .as_object()
                    .is_some_and(|object| object.contains_key(name))
            }),
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                property_names,
                property_count,
                ..
            } => {
                property_count.max() == Some(0)
                    || property_count.max().is_some_and(|max| {
                        let guaranteed = guaranteed_property_name_closure(schema);
                        !guaranteed.contains(name) && guaranteed.len() >= max
                    })
                    || properties
                        .get(name)
                        .is_some_and(schema_is_locally_empty_for_finite_enumeration)
                    || pattern_properties.values().any(|pattern_property| {
                        pattern_property.pattern.support() == PatternSupport::Supported
                            && pattern_property.pattern.is_match(name)
                            && schema_is_locally_empty_for_finite_enumeration(
                                &pattern_property.schema,
                            )
                    })
                    || string_literal_definitely_rejected(property_names, name)
                    || (!properties.contains_key(name)
                        && pattern_properties.values().all(|pattern_property| {
                            pattern_property.pattern.support() == PatternSupport::Supported
                                && !pattern_property.pattern.is_match(name)
                        })
                        && schema_is_locally_empty_for_finite_enumeration(additional))
            }
            SchemaNodeKind::AllOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty() && children.iter().all(|child| inner(child, name, active))
            }
            SchemaNodeKind::IfThenElse {
                then_schema,
                else_schema,
                ..
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, active) && inner(else_schema, name, active)
                }
                _ => false,
            },
            SchemaNodeKind::Not(child) => schema_accepts_all_objects_with_property(child, name),
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, name, &mut HashSet::new())
}

/// Object-scoped variant of `schema_forbids_property_name`.  It may return
/// true for schemas that also accept scalars, so callers must first establish
/// that only object values can overlap with the comparison side.
pub(super) fn schema_forbids_property_name_for_objects(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) => false,
            SchemaNodeKind::Const(value) => value
                .as_object()
                .is_none_or(|object| !object.contains_key(name)),
            SchemaNodeKind::Enum(values) => {
                let mut saw_object = false;
                let mut ok = true;
                for value in values {
                    if let Some(object) = value.as_object() {
                        saw_object = true;
                        if object.contains_key(name) {
                            ok = false;
                            break;
                        }
                    }
                }
                !saw_object || ok
            }
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                property_names,
                property_count,
                ..
            } => {
                property_count.max() == Some(0)
                    || property_count.max().is_some_and(|max| {
                        let guaranteed = guaranteed_property_name_closure(schema);
                        !guaranteed.contains(name) && guaranteed.len() >= max
                    })
                    || properties
                        .get(name)
                        .is_some_and(schema_is_locally_empty_for_finite_enumeration)
                    || pattern_properties.values().any(|pattern_property| {
                        pattern_property.pattern.support() == PatternSupport::Supported
                            && pattern_property.pattern.is_match(name)
                            && schema_is_locally_empty_for_finite_enumeration(
                                &pattern_property.schema,
                            )
                    })
                    || string_literal_definitely_rejected(property_names, name)
                    || (!properties.contains_key(name)
                        && pattern_properties.values().all(|pattern_property| {
                            pattern_property.pattern.support() == PatternSupport::Supported
                                && !pattern_property.pattern.is_match(name)
                        })
                        && schema_is_locally_empty_for_finite_enumeration(additional))
            }
            SchemaNodeKind::AllOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut saw_object_branch = false;
                let mut ok = true;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_OBJECT == 0 {
                        continue;
                    }
                    saw_object_branch = true;
                    if !inner(child, name, active) {
                        ok = false;
                        break;
                    }
                }
                !saw_object_branch || ok
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, active) && inner(else_schema, name, active)
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    inner(then_schema, name, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    inner(else_schema, name, active)
                }
                _ => false,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }

    schema_forbids_property_name(schema, name) || inner(schema, name, &mut HashSet::new())
}

pub(super) fn required_vs_forbidden_property_are_disjoint(
    left: &SchemaNode,
    right: &SchemaNode,
) -> bool {
    fn guaranteed_names(
        schema: &SchemaNode,
        out: &mut HashSet<String>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::Object { required, .. } => out.extend(required.iter().cloned()),
            SchemaNodeKind::Const(value) => {
                if let Some(object) = value.as_object() {
                    out.extend(object.keys().cloned());
                }
            }
            SchemaNodeKind::Enum(values) if !values.is_empty() => {
                let mut iter = values.iter();
                if let Some(first) = iter.next().and_then(Value::as_object) {
                    let mut common: HashSet<String> = first.keys().cloned().collect();
                    for value in iter {
                        if let Some(object) = value.as_object() {
                            common.retain(|name| object.contains_key(name));
                        } else {
                            common.clear();
                            break;
                        }
                    }
                    out.extend(common);
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    guaranteed_names(child, out, active);
                }
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children)
                if !children.is_empty() =>
            {
                let mut iter = children.iter();
                let mut common = HashSet::new();
                guaranteed_names(iter.next().expect("nonempty"), &mut common, active);
                for child in iter {
                    let mut child_names = HashSet::new();
                    guaranteed_names(child, &mut child_names, active);
                    common.retain(|name| child_names.contains(name));
                }
                out.extend(common);
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    let mut common = HashSet::new();
                    guaranteed_names(then_schema, &mut common, active);
                    let mut else_names = HashSet::new();
                    guaranteed_names(else_schema, &mut else_names, active);
                    common.retain(|name| else_names.contains(name));
                    out.extend(common);
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    guaranteed_names(then_schema, out, active);
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    guaranteed_names(else_schema, out, active);
                }
                _ => {}
            },
            _ => {}
        }
        active.remove(&schema.id());
    }

    let object_only_overlap =
        (possible_json_type_mask(left) & possible_json_type_mask(right) & !JSON_TYPE_OBJECT) == 0;
    let guarantees = |schema: &SchemaNode, name: &str| {
        schema_guarantees_property_name(schema, name)
            || (object_only_overlap && schema_guarantees_property_name_for_objects(schema, name))
    };
    let forbids = |schema: &SchemaNode, name: &str| {
        schema_forbids_property_name(schema, name)
            || (object_only_overlap && schema_forbids_property_name_for_objects(schema, name))
    };

    let mut left_names = guaranteed_property_name_closure(left);
    guaranteed_names(left, &mut left_names, &mut HashSet::new());
    if left_names
        .iter()
        .any(|name| guarantees(left, name) && forbids(right, name))
    {
        return true;
    }
    let mut right_names = guaranteed_property_name_closure(right);
    guaranteed_names(right, &mut right_names, &mut HashSet::new());
    right_names
        .iter()
        .any(|name| guarantees(right, name) && forbids(left, name))
}

pub(super) fn required_property_values_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    let left_values = finite_required_property_value_bounds(left);
    let right_values = finite_required_property_value_bounds(right);
    for (name, left_bound) in left_values {
        let Some(right_bound) = right_values.get(&name) else {
            continue;
        };
        if left_bound.is_empty() || right_bound.is_empty() {
            return true;
        }
        if left_bound.iter().all(|left_value| {
            right_bound
                .iter()
                .all(|right_value| !json_values_equal(left_value, right_value))
        }) {
            return true;
        }
    }
    required_vs_forbidden_property_are_disjoint(left, right)
        || required_property_schema_shapes_are_disjoint(left, right)
}

/// Return true when both object schemas force the same property and the forced
/// property's schemas have obviously disjoint primitive domains. This is a
/// deliberately smaller fact than full property-schema implication: it avoids
/// recursively invoking object disjointness (which can cycle through `$ref`s),
/// but still catches common tagged unions where the tag is separated by JSON
/// type or a simple numeric interval rather than by finite const/enum values.
pub(super) fn required_property_schema_shapes_are_disjoint(
    left: &SchemaNode,
    right: &SchemaNode,
) -> bool {
    fn guaranteed_names(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> HashSet<String> {
        if !active.insert(schema.id()) {
            return HashSet::new();
        }
        // Start with the dependency closure for this whole node. In an allOf,
        // a required trigger in one conjunct can activate a dependentRequired
        // rule in another; those implied names are just as guaranteed as
        // syntactic `required` names for discriminator-shape checks.
        let mut names = guaranteed_property_name_closure(schema);
        match schema.kind() {
            SchemaNodeKind::Object { .. } => {}
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    names.extend(guaranteed_names(child, active));
                }
            }
            // A union only guarantees names common to every branch. This is a
            // useful safe case, but avoid treating an empty union as universal.
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children)
                if !children.is_empty() =>
            {
                let mut iter = children.iter();
                let mut common = guaranteed_names(iter.next().expect("nonempty"), active);
                for child in iter {
                    let child_names = guaranteed_names(child, active);
                    common.retain(|name| child_names.contains(name));
                }
                names.extend(common);
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    names.extend(guaranteed_names(then_schema, active));
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    names.extend(guaranteed_names(else_schema, active));
                }
                (Some(then_schema), Some(else_schema)) => {
                    let mut common = guaranteed_names(then_schema, active);
                    let else_names = guaranteed_names(else_schema, active);
                    common.retain(|name| else_names.contains(name));
                    names.extend(common);
                }
                _ => {}
            },
            _ => {}
        }
        active.remove(&schema.id());
        names
    }

    fn collect_constraints_for_name<'a>(
        schema: &'a SchemaNode,
        name: &str,
        out: &mut Vec<&'a SchemaNode>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                ..
            } => {
                let mut matched = false;
                if let Some(property_schema) = properties.get(name) {
                    matched = true;
                    out.push(property_schema);
                }
                let mut unsupported_pattern = false;
                for pattern_property in pattern_properties.values() {
                    if pattern_property.pattern.support() != PatternSupport::Supported {
                        unsupported_pattern = true;
                        continue;
                    }
                    if pattern_property.pattern.is_match(name) {
                        matched = true;
                        out.push(&pattern_property.schema);
                    }
                }
                if !matched && !unsupported_pattern {
                    out.push(additional);
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    collect_constraints_for_name(child, name, out, active);
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    collect_constraints_for_name(then_schema, name, out, active);
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    collect_constraints_for_name(else_schema, name, out, active);
                }
                _ => {}
            },
            // For unions, a constraint applies to every accepted value only if
            // every branch contributes an obvious constraint for this name. We
            // do not try to merge those alternatives here; finite discriminator
            // extraction handles the common const/enum case separately.
            _ => {}
        }
        active.remove(&schema.id());
    }

    fn constraint_map(schema: &SchemaNode) -> HashMap<String, Vec<&SchemaNode>> {
        let names = guaranteed_names(schema, &mut HashSet::new());
        let mut result = HashMap::new();
        for name in names {
            let mut constraints = Vec::new();
            collect_constraints_for_name(schema, &name, &mut constraints, &mut HashSet::new());
            if !constraints.is_empty() {
                result.insert(name, constraints);
            }
        }
        result
    }

    let left_constraints = constraint_map(left);
    let right_constraints = constraint_map(right);

    left_constraints.iter().any(|(name, left_schemas)| {
        let Some(right_schemas) = right_constraints.get(name) else {
            return false;
        };
        left_schemas.iter().any(|left_schema| {
            right_schemas
                .iter()
                .any(|right_schema| primitive_domains_are_disjoint(left_schema, right_schema))
        })
    })
}

/// A deliberately small primitive-domain disjointness witness used inside
/// object and tuple discriminators. It never recurses into object/array shape
/// reasoning, which keeps `$ref` cycles from turning a cheap partition fact
/// into an unbounded proof search.
pub(super) fn primitive_domains_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    if let (Some(left_values), Some(right_values)) = (
        finite_schema_value_superset(left),
        finite_schema_value_superset(right),
    ) && left_values.iter().all(|left_value| {
        right_values
            .iter()
            .all(|right_value| !json_values_equal(left_value, right_value))
    }) {
        return true;
    }

    let left_mask = possible_json_type_mask(left);
    let right_mask = possible_json_type_mask(right);
    let overlap = left_mask & right_mask;
    if overlap == 0 {
        return true;
    }
    (overlap == JSON_TYPE_NUMBER && numeric_intervals_are_disjoint(left, right))
        || (overlap == JSON_TYPE_STRING && string_length_intervals_are_disjoint(left, right))
        || (overlap == JSON_TYPE_ARRAY && array_length_intervals_are_disjoint(left, right))
        || (overlap == JSON_TYPE_OBJECT
            && object_property_count_intervals_are_disjoint(left, right))
}

/// Return true when two array schemas both force an item at the same tuple
/// position and the schemas applying to that item have disjoint primitive
/// domains. This catches tagged tuple unions without attempting general
/// item-subset reasoning.
pub(super) fn required_array_item_shapes_are_disjoint(
    left: &SchemaNode,
    right: &SchemaNode,
) -> bool {
    const MAX_TRACKED_PREFIX: usize = 32;

    fn guaranteed_len(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> usize {
        if !active.insert(schema.id()) {
            return 0;
        }
        let len = match schema.kind() {
            SchemaNodeKind::Array { item_count, .. } => {
                usize::try_from(item_count.min()).unwrap_or(usize::MAX)
            }
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .map(|child| guaranteed_len(child, active))
                .max()
                .unwrap_or(0),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children)
                if !children.is_empty() =>
            {
                children
                    .iter()
                    .map(|child| guaranteed_len(child, active))
                    .min()
                    .unwrap_or(0)
            }
            SchemaNodeKind::Const(value) => value.as_array().map_or(0, Vec::len),
            SchemaNodeKind::Enum(values) if !values.is_empty() => values
                .iter()
                .map(|value| value.as_array().map_or(0, Vec::len))
                .min()
                .unwrap_or(0),
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    guaranteed_len(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    guaranteed_len(else_schema, active)
                }
                (Some(then_schema), Some(else_schema)) => {
                    guaranteed_len(then_schema, active).min(guaranteed_len(else_schema, active))
                }
                _ => 0,
            },
            _ => 0,
        };
        active.remove(&schema.id());
        len.min(MAX_TRACKED_PREFIX)
    }

    fn collect_constraints_at<'a>(
        schema: &'a SchemaNode,
        index: usize,
        out: &mut Vec<&'a SchemaNode>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::Array {
                prefix_items,
                items,
                ..
            } => {
                if let Some(prefix) = prefix_items.get(index) {
                    out.push(prefix);
                } else {
                    out.push(items);
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    collect_constraints_at(child, index, out, active);
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    collect_constraints_at(then_schema, index, out, active);
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    collect_constraints_at(else_schema, index, out, active);
                }
                // With two live branches, constraints are alternatives rather
                // than conjuncts; leave those to finite discriminator helpers.
                _ => {}
            },
            _ => {}
        }
        active.remove(&schema.id());
    }

    let shared_len =
        guaranteed_len(left, &mut HashSet::new()).min(guaranteed_len(right, &mut HashSet::new()));
    for index in 0..shared_len {
        let mut left_constraints = Vec::new();
        let mut right_constraints = Vec::new();
        collect_constraints_at(left, index, &mut left_constraints, &mut HashSet::new());
        collect_constraints_at(right, index, &mut right_constraints, &mut HashSet::new());
        if left_constraints.iter().any(|left_item| {
            right_constraints
                .iter()
                .any(|right_item| primitive_domains_are_disjoint(left_item, right_item))
        }) {
            return true;
        }
    }
    false
}

/// For each returned property name, every object accepted by `schema` has that
/// property and its value is contained in the returned finite upper bound.
/// Missing entries mean "unknown", not unconstrained. This is intentionally
/// small: direct object `required`+`properties` constraints and allOf
/// propagation cover the usual discriminator shape without attempting object
/// satisfiability.
pub(super) fn finite_required_property_value_bounds(
    schema: &SchemaNode,
) -> HashMap<String, Vec<Value>> {
    fn intersect_value_bounds(left: &mut Vec<Value>, right: &[Value]) {
        left.retain(|left_value| {
            right
                .iter()
                .any(|right_value| json_values_equal(left_value, right_value))
        });
    }

    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> HashMap<String, Vec<Value>> {
        if !active.insert(schema.id()) {
            return HashMap::new();
        }
        let mut result = HashMap::new();
        match schema.kind() {
            SchemaNodeKind::Object { properties, .. } => {
                // Include names implied by dependentRequired closure, not just
                // syntactic `required`. If a forced name has a finite property
                // domain, it can serve as a discriminator too.
                for name in guaranteed_property_name_closure(schema) {
                    if let Some(property_schema) = properties.get(&name)
                        && let Some(values) = finite_schema_value_superset(property_schema)
                    {
                        result.insert(name, values);
                    }
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    for (name, mut values) in inner(child, active) {
                        match result.get_mut(&name) {
                            Some(existing) => intersect_value_bounds(existing, &values),
                            None => {
                                // Deduplicate for stable comparisons and to keep bounds small.
                                let mut deduped: Vec<Value> = Vec::new();
                                for value in values.drain(..) {
                                    if !deduped.iter().any(|seen| json_values_equal(seen, &value)) {
                                        deduped.push(value);
                                    }
                                }
                                result.insert(name, deduped);
                            }
                        }
                    }
                }
            }
            SchemaNodeKind::Const(value) => {
                if let Some(object) = value.as_object() {
                    for (name, property_value) in object {
                        result.insert(name.clone(), vec![property_value.clone()]);
                    }
                }
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                // A union guarantees a discriminator only when every branch
                // guarantees it.  The value bound is then the union of the
                // branch bounds.  This is useful for generated schemas that
                // wrap tagged objects in an anyOf/oneOf layer, and remains
                // conservative for empty or recursive unions.
                let mut child_iter = children.iter();
                if let Some(first_child) = child_iter.next() {
                    result = inner(first_child, active);
                    for child in child_iter {
                        let child_bounds = inner(child, active);
                        result.retain(|name, values| {
                            let Some(other_values) = child_bounds.get(name) else {
                                return false;
                            };
                            for value in other_values {
                                if !values.iter().any(|seen| json_values_equal(seen, value)) {
                                    values.push(value.clone());
                                }
                            }
                            true
                        });
                        if result.is_empty() {
                            break;
                        }
                    }
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    result = inner(then_schema, active);
                    let else_bounds = inner(else_schema, active);
                    result.retain(|name, values| {
                        let Some(other_values) = else_bounds.get(name) else {
                            return false;
                        };
                        for value in other_values {
                            if !values.iter().any(|seen| json_values_equal(seen, value)) {
                                values.push(value.clone());
                            }
                        }
                        true
                    });
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    result = inner(then_schema, active);
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    result = inner(else_schema, active);
                }
                _ => {}
            },
            SchemaNodeKind::Enum(values)
                if !values.is_empty() && values.iter().all(|value| value.as_object().is_some()) =>
            {
                // A property is guaranteed only if *every* enumerated value is
                // an object and has that property.  Mixed-type enums are common
                // in generated schemas; ignoring their non-object literals here
                // would make discriminator disjointness unsound.
                let first = values[0].as_object().expect("checked above");
                for (name, first_value) in first {
                    let mut bound = vec![first_value.clone()];
                    let mut guaranteed = true;
                    for object in values.iter().filter_map(Value::as_object) {
                        let Some(value) = object.get(name) else {
                            guaranteed = false;
                            break;
                        };
                        if !bound.iter().any(|seen| json_values_equal(seen, value)) {
                            bound.push(value.clone());
                        }
                    }
                    if guaranteed {
                        result.insert(name.clone(), bound);
                    }
                }
            }
            SchemaNodeKind::Enum(_) => {}
            _ => {}
        }
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}
