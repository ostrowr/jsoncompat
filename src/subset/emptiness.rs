//! Local universality and impossibility checks used as conservative proof shortcuts.
//!
//! These predicates are intentionally one-sided: `false` means unknown.

use super::*;

pub(super) fn schema_is_trivially_universal(schema: &SchemaNode) -> bool {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::Not(child) => matches!(child.kind(), SchemaNodeKind::BoolSchema(false)),
            SchemaNodeKind::AllOf(children) => children.iter().all(|child| inner(child, active)),
            SchemaNodeKind::AnyOf(children) => children.iter().any(|child| inner(child, active)),
            SchemaNodeKind::IfThenElse {
                then_schema,
                else_schema,
                ..
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, active) && inner(else_schema, active)
                }
                (Some(branch), None) | (None, Some(branch)) => inner(branch, active),
                (None, None) => true,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, &mut HashSet::new())
}

/// Non-recursive emptiness check used while constructing finite value supersets.
/// The full emptiness prover may recursively ask for another finite superset;
/// doing that from inside enumeration can re-enter recursive schemas with a
/// fresh visitation set. Keep this deliberately local.
pub(super) fn array_schema_is_locally_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::Array {
        prefix_items,
        items,
        item_count,
        contains,
        unique_items,
        enumeration,
    } = schema.kind()
    else {
        return false;
    };
    let constraints = array::ArrayConstraints {
        prefix_items,
        items,
        item_count: *item_count,
        contains: contains.as_ref(),
        unique_items: *unique_items,
        enumeration: enumeration.as_deref(),
    };
    if array::array_constraints_definitely_uninhabited(&constraints) {
        return true;
    }
    let Some(effective_count) = array::effective_item_count_for_unique_finite_domain(
        prefix_items,
        items,
        *item_count,
        *unique_items,
    ) else {
        return true;
    };
    if let Some(contains) = contains.as_ref() {
        let matcher_disjoint_from_every_position = contains.count().min() > 0
            && prefix_items
                .iter()
                .all(|item| schemas_definitely_disjoint_by_shape(item, &contains.schema))
            && schemas_definitely_disjoint_by_shape(items, &contains.schema);
        array::contains_requirement_definitely_impossible(contains, effective_count, *unique_items)
            || array::contains_requirement_impossible_for_unique_finite_items(
                prefix_items,
                items,
                effective_count,
                contains,
                *unique_items,
            )
            || matcher_disjoint_from_every_position
            || array_contains_requirement_is_locally_impossible(schema)
    } else {
        false
    }
}

pub(super) fn object_schema_is_locally_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::Object {
        properties,
        pattern_properties,
        required,
        additional,
        property_names,
        property_count,
        dependent_required,
        enumeration,
        ..
    } = schema.kind()
    else {
        return false;
    };
    object::object_constraints_definitely_uninhabited(&object::ObjectConstraints {
        properties,
        pattern_properties,
        required,
        additional,
        property_names,
        property_count: *property_count,
        dependent_required,
        enumeration: enumeration.as_deref(),
    })
}

pub(super) fn array_contains_requirement_is_locally_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::Array {
        contains: Some(contains),
        item_count,
        ..
    } = schema.kind()
    else {
        return false;
    };
    let required = contains.count().min();
    if required == 0 {
        return false;
    }
    if item_count
        .max()
        .is_some_and(|max_items| required > max_items)
    {
        return true;
    }
    schema_is_locally_empty_for_finite_enumeration(&contains.schema)
}

/// Detect a split `allOf` object contradiction where a property is guaranteed
/// present and two conjuncts give it disjoint direct `properties` schemas.
/// This stays deliberately syntactic; pattern/additionalProperties interactions
/// are left to the existing object-local prover.
pub(super) fn split_allof_object_property_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(children) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_OBJECT != 0 {
        return false;
    }

    let mut required = HashSet::<String>::new();
    let mut constraints: HashMap<String, Vec<&SchemaNode>> = HashMap::new();
    for child in children {
        let child = unwrap_singleton_applicators(child);
        if let SchemaNodeKind::Object {
            properties,
            required: child_required,
            ..
        } = child.kind()
        {
            required.extend(child_required.iter().cloned());
            for (name, property_schema) in properties {
                constraints
                    .entry(name.clone())
                    .or_default()
                    .push(property_schema);
            }
        }
    }

    for name in required {
        let Some(property_constraints) = constraints.get(&name) else {
            continue;
        };
        for i in 0..property_constraints.len() {
            for j in (i + 1)..property_constraints.len() {
                if schemas_definitely_disjoint_by_shape(
                    property_constraints[i],
                    property_constraints[j],
                ) {
                    return true;
                }
            }
        }
    }
    false
}

/// Detect a split `allOf` array contradiction where one conjunct constrains
/// every item to a homogeneous schema and another requires a `contains` match
/// that is disjoint from that homogeneous item schema.  This is intentionally
/// narrow: the whole intersection must be array-only, and the homogeneous
/// conjunct must have no prefix tuple holes.
pub(super) fn split_allof_array_contains_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(children) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_ARRAY != 0 {
        return false;
    }

    let mut homogeneous_items: Vec<&SchemaNode> = Vec::new();
    let mut required_contains: Vec<&SchemaNode> = Vec::new();
    for child in children {
        let child = unwrap_singleton_applicators(child);
        if let SchemaNodeKind::Array {
            prefix_items,
            items,
            contains,
            ..
        } = child.kind()
        {
            if prefix_items.is_empty() {
                homogeneous_items.push(items);
            }
            if let Some(contains) = contains.as_ref()
                && contains.count().min() > 0
            {
                required_contains.push(&contains.schema);
            }
        }
    }

    homogeneous_items.iter().any(|items| {
        required_contains
            .iter()
            .any(|matcher| schemas_definitely_disjoint_by_shape(items, matcher))
    })
}

/// Detect a split `allOf` array length/count contradiction.
pub(super) fn split_allof_array_length_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(_) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_ARRAY != 0 {
        return false;
    }
    array_length_interval_bound(schema).is_some_and(|interval| interval.empty)
}

/// Detect a split `allOf` object property-count contradiction.
pub(super) fn split_allof_object_count_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(_) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_OBJECT != 0 {
        return false;
    }
    object_property_count_interval_bound(schema).is_some_and(|interval| interval.empty)
}

/// Detect a split `allOf` numeric-range contradiction.  This only fires when
/// the whole intersection is numeric-only and the existing interval extractor
/// can prove the intersection empty, so lattice/multipleOf cases remain with
/// the more precise numeric provers.
pub(super) fn split_allof_numeric_range_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(_) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_NUMBER != 0 {
        return false;
    }
    numeric_interval_bound(schema).is_some_and(|interval| interval.empty)
}

/// Detect the simplest split `allOf` string-length contradiction.  The normal
/// string interval reasoning is used for pairwise disjointness, but negation
/// normalization needs an explicit emptiness witness for schemas such as
/// `allOf: [{minLength: 3}, {maxLength: 1}]`.  Keep this syntactic and
/// string-only so it cannot accidentally classify a mixed-type intersection as
/// empty.
pub(super) fn split_allof_string_length_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(children) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_STRING != 0 {
        return false;
    }

    let mut lower = 0_u64;
    let mut upper: Option<u64> = None;
    let mut saw_string_bound = false;
    for child in children {
        let child = unwrap_singleton_applicators(child);
        let SchemaNodeKind::String { length, .. } = child.kind() else {
            continue;
        };
        saw_string_bound = true;
        lower = lower.max(length.min());
        if let Some(child_upper) = length.max() {
            upper = Some(upper.map_or(child_upper, |current| current.min(child_upper)));
        }
    }

    saw_string_bound && upper.is_some_and(|max_len| lower > max_len)
}

/// A deliberately small emptiness predicate for the inner side of a negation.
///
/// `not(false)`-like targets are universal, but the finite-enumeration
/// emptiness helper intentionally does not look at the richer array/object
/// contradiction checks (unique finite domains, impossible `contains`, closed
/// object/property-name conflicts). Those checks are already used to prove an
/// impossible schema is a subset of an arbitrary typed target; expose the same
/// fact to negation normalization without turning this into a general recursive
/// emptiness prover. The applicator recursion below only follows directions
/// that preserve emptiness, and is cycle guarded.
pub(super) fn schema_is_locally_impossible_for_negation(schema: &SchemaNode) -> bool {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let normalized = unwrap_singleton_applicators(schema);
        if normalized.id() != schema.id() {
            let result = inner(normalized, active);
            active.remove(&schema.id());
            return result;
        }

        let result = schema_is_locally_empty_for_finite_enumeration(schema)
            || split_allof_array_contains_is_impossible(schema)
            || split_allof_object_property_is_impossible(schema)
            || split_allof_array_length_is_impossible(schema)
            || split_allof_object_count_is_impossible(schema)
            || split_allof_numeric_range_is_impossible(schema)
            || split_allof_string_length_is_impossible(schema)
            || array_schema_is_locally_impossible(schema)
            || object_schema_is_locally_impossible(schema)
            || match schema.kind() {
                SchemaNodeKind::AllOf(children) => {
                    children.iter().any(|child| inner(child, active))
                }
                SchemaNodeKind::AnyOf(children) => {
                    !children.is_empty() && children.iter().all(|child| inner(child, active))
                }
                SchemaNodeKind::OneOf(children) => {
                    (!children.is_empty() && children.iter().all(|child| inner(child, active)))
                        || (children.len() == 2
                            && schemas_obviously_equivalent(&children[0], &children[1]))
                }
                SchemaNodeKind::IfThenElse {
                    if_schema,
                    then_schema,
                    else_schema,
                } => {
                    // If both explicit branches are empty, no guard outcome can
                    // admit a value.  For a locally constant guard, it is also
                    // enough for the selected explicit branch to be empty;
                    // a missing selected branch is implicit `true`, so it is
                    // deliberately not treated as empty.
                    match (then_schema.as_ref(), else_schema.as_ref()) {
                        (Some(then_branch), Some(else_branch))
                            if inner(then_branch, active) && inner(else_branch, active) =>
                        {
                            true
                        }
                        (Some(then_branch), _) if schema_is_trivially_universal(if_schema) => {
                            inner(then_branch, active)
                        }
                        (_, Some(else_branch)) if inner(if_schema, active) => {
                            inner(else_branch, active)
                        }
                        _ => false,
                    }
                }
                _ => false,
            };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

pub(super) fn schema_is_locally_empty_for_finite_enumeration(schema: &SchemaNode) -> bool {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            // Recursive wrappers are not a local emptiness witness by themselves.
            return false;
        }
        let result = if matches!(schema.kind(), SchemaNodeKind::BoolSchema(false))
            || matches!(schema.kind(), SchemaNodeKind::Enum(values) if values.is_empty())
            || possible_json_type_mask(schema) == 0
            || matches!(constrained_enumeration(schema), Some(values) if values.is_empty())
        {
            true
        } else if let Some(values) = constrained_enumeration(schema)
            && !schema_may_under_accept_values(schema)
            && values.iter().all(|value| !schema.accepts_value(value))
        {
            true
        } else {
            match schema.kind() {
                SchemaNodeKind::AllOf(children) => {
                    children.iter().any(|child| inner(child, active))
                }
                SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children)
                    if children.len() == 1 =>
                {
                    inner(&children[0], active)
                }
                _ => false,
            }
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}
