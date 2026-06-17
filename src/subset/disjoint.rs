//! Conservative disjointness and negation-cover proofs.
//!
//! These routines prove only one-sided facts; `false` always means unknown.

use super::*;

pub(super) fn schemas_definitely_disjoint_for_negation(
    left: &SchemaNode,
    right: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn inner(
        left: &SchemaNode,
        right: &SchemaNode,
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<(NodeId, NodeId)>,
    ) -> bool {
        if !active.insert((left.id(), right.id())) {
            return false;
        }

        // `possible_json_type_mask` is a sound upper bound on accepted JSON
        // types.  If two upper bounds are disjoint, the languages themselves
        // are disjoint, regardless of any applicator structure inside either
        // side.  This cheap check is especially useful when proving
        // `S <= not(anyOf[T...])`: each excluded arm can often be ruled out by
        // type alone before we descend into contains/items/property details.
        let left_mask = possible_json_type_mask(left);
        let right_mask = possible_json_type_mask(right);
        if left_mask == 0 || right_mask == 0 || (left_mask & right_mask) == 0 {
            active.remove(&(left.id(), right.id()));
            return true;
        }

        // If either side has a finite upper bound, concrete-value rejection can
        // prove disjointness even when the other side is an explicit
        // complement/intersection (for example `allOf` of `not const` versus
        // an `enum`).  Because the finite set is only an upper bound, each
        // candidate must either be definitely rejected by its own schema or by
        // the opposite schema.
        if let Some(values) = finite_schema_value_superset(left)
            && values.iter().all(|value| {
                context.schema_definitely_rejects_value(left, value)
                    || context.schema_definitely_rejects_value(right, value)
            })
        {
            active.remove(&(left.id(), right.id()));
            return true;
        }
        if let Some(values) = finite_schema_value_superset(right)
            && values.iter().all(|value| {
                context.schema_definitely_rejects_value(right, value)
                    || context.schema_definitely_rejects_value(left, value)
            })
        {
            active.remove(&(left.id(), right.id()));
            return true;
        }

        use SchemaNodeKind::*;
        let result = match (left.kind(), right.kind()) {
            (BoolSchema(false), _) | (_, BoolSchema(false)) => true,
            // Distribute the cheap disjointness proof across an intersection
            // versus a union: every union arm must be ruled out, and any
            // single conjunct can rule out a given arm.  The generic AllOf
            // and AnyOf cases below are intentionally simpler; this mixed
            // shape is common after spelling De Morgan complements explicitly.
            (AllOf(conjuncts), AnyOf(branches)) | (AllOf(conjuncts), OneOf(branches)) => {
                branches.iter().all(|branch| {
                    conjuncts
                        .iter()
                        .any(|conjunct| inner(conjunct, branch, context, active))
                })
            }
            (AnyOf(branches), AllOf(conjuncts)) | (OneOf(branches), AllOf(conjuncts)) => {
                branches.iter().all(|branch| {
                    conjuncts
                        .iter()
                        .any(|conjunct| inner(branch, conjunct, context, active))
                })
            }
            // A complement is disjoint from any schema proved to be contained
            // by the complemented language. This is the small implication
            // needed for De Morgan-shaped schemas such as
            // `allOf: [{not: A}, {not: B}]` versus `not: {anyOf: [A, B]}`:
            // when checking disjointness with one branch `A`, the explicit
            // `not: A` conjunct is enough. Keep this as an implication proof
            // rather than attempting general complement algebra.
            (Not(excluded), _) => {
                analyze_subschema_with_context(
                    right,
                    excluded,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
            }
            (_, Not(excluded)) => {
                analyze_subschema_with_context(
                    left,
                    excluded,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
            }
            (AnyOf(children), _) | (OneOf(children), _) => children
                .iter()
                .all(|child| inner(child, right, context, active)),
            (AllOf(children), _) => children
                .iter()
                .any(|child| inner(child, right, context, active)),
            (_, AnyOf(children)) | (_, OneOf(children)) => children
                .iter()
                .all(|child| inner(left, child, context, active)),
            (_, AllOf(children)) => children
                .iter()
                .any(|child| inner(left, child, context, active)),
            _ => {
                let left_mask = possible_json_type_mask(left);
                left_mask == 0
                    || schemas_definitely_disjoint_for_partition(left, left_mask, right, context)
            }
        };

        active.remove(&(left.id(), right.id()));
        result
    }

    inner(left, right, context, &mut HashSet::new())
}

/// Sufficient De Morgan cover for `not(allOf: [A, B, ...])` against an
/// `anyOf` target.  The complement of an intersection is the union of the
/// complements; for each conjunct `A`, it is enough to find a target branch
/// `not B` with `B <= A` (contravariance of negation).  This deliberately
/// avoids constructing synthetic schema nodes and stays conservative for
/// non-negated target branches.
pub(super) fn negated_allof_covered_by_anyof(
    conjuncts: &[SchemaNode],
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    conjuncts.iter().all(|conjunct| {
        branches.iter().any(|branch| match branch.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::Not(excluded) => {
                analyze_subschema_with_context(
                    excluded,
                    conjunct,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
            }
            _ => false,
        })
    })
}
/// Prove `not(anyOf([...])) <= target` when one excluded-union arm is a
/// complement of a finite schema. If an arm is `not A`, any value surviving
/// the outer negation must be in `A` and must avoid every sibling arm. Enumerate
/// an upper bound for `A`; each candidate must either be impossible for `A`, be
/// definitely consumed by a sibling arm, or be definitely accepted by target.
pub(super) fn negated_anyof_finite_complement_arm_subset_of(
    children: &[SchemaNode],
    target: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    children.iter().enumerate().any(|(neg_index, child)| {
        let SchemaNodeKind::Not(core) = child.kind() else {
            return false;
        };
        let Some(values) = finite_schema_value_superset(core) else {
            return false;
        };

        values.iter().all(|value| {
            context.schema_definitely_rejects_value(core, value)
                || children.iter().enumerate().any(|(index, sibling)| {
                    index != neg_index && context.superset_contains_value(sibling, value)
                })
                || context.superset_contains_value(target, value)
        })
    })
}

/// Cover `not A` by an `anyOf` that contains a coarser finite complement
/// `not B` plus explicit branches for the finite gap `B \ A`.
///
/// For any value accepted by `not A`, either it is outside `B` (and therefore
/// accepted by the `not B` branch) or it is one of the finite candidates in an
/// upper bound for `B`.  For those candidates, require either definite
/// acceptance by `A` (so they are irrelevant to `not A`) or definite acceptance
/// by some union branch.  This handles generated partitions such as
/// `not {const: 1}` being covered by `anyOf: [not {enum:[1,2]}, {const:2}]`.
pub(super) fn negated_exclusion_covered_by_anyof_finite_gap(
    excluded: &SchemaNode,
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    branches.iter().any(|branch| {
        let SchemaNodeKind::Not(coarser_excluded) = branch.kind() else {
            return false;
        };
        let Some(values) = finite_schema_value_superset(coarser_excluded) else {
            return false;
        };

        values.iter().all(|value| {
            context.schema_definitely_rejects_value(coarser_excluded, value)
                || context.superset_contains_value(excluded, value)
                || branches
                    .iter()
                    .any(|cover| context.superset_contains_value(cover, value))
        })
    })
}

/// Like `schemas_definitely_disjoint_for_oneof`, but also uses finite value
/// upper bounds with the concrete-value evaluator. This is only used while a
/// context is already available (for oneOf partition proofs): if every value
/// in a finite upper bound for either side is definitely rejected by the other
/// side, the languages cannot overlap. The upper-bound direction is important
/// here; unsupported schemas simply return `None` and keep the check
/// conservative.
pub(super) fn schema_contains_explicit_not(schema: &SchemaNode) -> bool {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::Not(_) => true,
            SchemaNodeKind::AllOf(children)
            | SchemaNodeKind::AnyOf(children)
            | SchemaNodeKind::OneOf(children) => children.iter().any(|child| inner(child, active)),
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                inner(if_schema, active)
                    || then_schema
                        .as_ref()
                        .is_some_and(|child| inner(child, active))
                    || else_schema
                        .as_ref()
                        .is_some_and(|child| inner(child, active))
            }
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, &mut HashSet::new())
}

pub(super) fn schemas_definitely_disjoint_for_partition(
    sub: &SchemaNode,
    sub_mask: u8,
    other: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn base(
        sub: &SchemaNode,
        sub_mask: u8,
        other: &SchemaNode,
        context: &mut SubschemaCheckContext,
    ) -> bool {
        if schemas_definitely_disjoint_for_oneof(sub, sub_mask, other) {
            return true;
        }
        if let Some(values) = finite_schema_value_superset(sub)
            && values
                .iter()
                .all(|value| context.schema_definitely_rejects_value(other, value))
        {
            return true;
        }
        if let Some(values) = finite_schema_value_superset(other)
            && values
                .iter()
                .all(|value| context.schema_definitely_rejects_value(sub, value))
        {
            return true;
        }
        let overlap = possible_json_type_mask(sub) & possible_json_type_mask(other);
        // Property/tuple witnesses only prove disjointness for the object/array
        // portions of the languages.  If the same pair can also overlap on a
        // scalar (or on the other container kind), treating that local witness
        // as whole-schema disjointness would make oneOf partitions unsound.
        if overlap != 0
            && overlap & !JSON_TYPE_OBJECT == 0
            && (finite_required_property_values_rejected_by_other(sub, other, context)
                || finite_required_property_values_rejected_by_other(other, sub, context))
        {
            return true;
        }
        if overlap != 0
            && overlap & !JSON_TYPE_ARRAY == 0
            && (finite_required_array_item_values_rejected_by_other(sub, other, context)
                || finite_required_array_item_values_rejected_by_other(other, sub, context))
        {
            return true;
        }
        // Explicit complement guards are a common way generators spell the
        // "other" side of a partition (for example `allOf: [{not: A}, B]`).
        // Only invoke the recursive exclusion prover when a visible `not` is
        // present, both to keep this fast and to avoid re-entering the general
        // subset checker for ordinary branch pairs.
        if (schema_contains_explicit_not(sub)
            && schema_definitely_excludes_schema(sub, other, context))
            || (schema_contains_explicit_not(other)
                && schema_definitely_excludes_schema(other, sub, context))
        {
            return true;
        }
        false
    }

    fn inner(
        left: &SchemaNode,
        right: &SchemaNode,
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<(NodeId, NodeId)>,
    ) -> bool {
        if !active.insert((left.id(), right.id())) {
            return false;
        }
        use SchemaNodeKind::*;
        // Try the whole pair first; split-allOf discriminator witnesses often
        // require constraints from multiple conjuncts.
        let result = if base(left, possible_json_type_mask(left), right, context) {
            true
        } else {
            match (left.kind(), right.kind()) {
                (BoolSchema(false), _) | (_, BoolSchema(false)) => true,
                (AnyOf(children), _) | (OneOf(children), _) => children
                    .iter()
                    .all(|child| inner(child, right, context, active)),
                (AllOf(children), _) => children
                    .iter()
                    .any(|child| inner(child, right, context, active)),
                (_, AnyOf(children)) | (_, OneOf(children)) => children
                    .iter()
                    .all(|child| inner(left, child, context, active)),
                (_, AllOf(children)) => children
                    .iter()
                    .any(|child| inner(left, child, context, active)),
                _ => false,
            }
        };
        active.remove(&(left.id(), right.id()));
        result
    }

    // Preserve the caller-provided mask for the common non-applicator fast path;
    // recursive applicator decomposition recomputes child masks as needed.
    if !matches!(
        sub.kind(),
        SchemaNodeKind::AnyOf(_) | SchemaNodeKind::OneOf(_) | SchemaNodeKind::AllOf(_)
    ) && !matches!(
        other.kind(),
        SchemaNodeKind::AnyOf(_) | SchemaNodeKind::OneOf(_) | SchemaNodeKind::AllOf(_)
    ) {
        return base(sub, sub_mask, other, context);
    }
    inner(sub, other, context, &mut HashSet::new())
}
/// Return true when `finite_side` forces a property into a finite set of
/// values, while `other` both guarantees that property exists and has a
/// syntactic property constraint that rejects every value in the set. This is
/// a context-aware sibling of the cheap discriminator checks: it lets the
/// concrete evaluator handle patterns/ranges on the property schema without
/// attempting general object implication.
pub(super) fn finite_required_property_values_rejected_by_other(
    finite_side: &SchemaNode,
    other: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    for (name, values) in finite_required_property_value_bounds(finite_side) {
        if values.is_empty() {
            return true;
        }
        let object_only_overlap = (possible_json_type_mask(finite_side)
            & possible_json_type_mask(other)
            & !JSON_TYPE_OBJECT)
            == 0;
        if !(schema_guarantees_property_name(other, &name)
            || (object_only_overlap && schema_guarantees_property_name_for_objects(other, &name)))
        {
            continue;
        }
        if property_value_set_definitely_rejected(other, &name, &values, context) {
            return true;
        }
    }
    false
}

/// Does `schema` reject every object that contains `name` with a value from
/// `values`? The caller is responsible for proving the property is present.
/// We only use local property applicators (plus safe applicator composition),
/// so a false result simply means "unknown".
pub(super) fn property_value_set_definitely_rejected(
    schema: &SchemaNode,
    name: &str,
    values: &[Value],
    context: &mut SubschemaCheckContext,
) -> bool {
    fn all_values_rejected(
        constraint: &SchemaNode,
        values: &[Value],
        context: &mut SubschemaCheckContext,
    ) -> bool {
        values
            .iter()
            .all(|value| context.schema_definitely_rejects_value(constraint, value))
    }

    fn inner(
        schema: &SchemaNode,
        name: &str,
        values: &[Value],
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<NodeId>,
    ) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) => false,
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                property_names,
                ..
            } => {
                if string_literal_definitely_rejected(property_names, name) {
                    true
                } else {
                    let mut matched = false;
                    let direct_rejects = properties.get(name).is_some_and(|property_schema| {
                        matched = true;
                        all_values_rejected(property_schema, values, context)
                    });
                    if direct_rejects {
                        true
                    } else {
                        let mut pattern_rejects = false;
                        let mut unsupported_pattern = false;
                        for pattern_property in pattern_properties.values() {
                            if pattern_property.pattern.support() != PatternSupport::Supported {
                                // It may match this name, in which case `additionalProperties`
                                // would not apply. Keep that fallback conservative.
                                unsupported_pattern = true;
                                continue;
                            }
                            if pattern_property.pattern.is_match(name) {
                                matched = true;
                                if all_values_rejected(&pattern_property.schema, values, context) {
                                    pattern_rejects = true;
                                    break;
                                }
                            }
                        }
                        pattern_rejects
                            || (!matched
                                && !unsupported_pattern
                                && all_values_rejected(additional, values, context))
                    }
                }
            }
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .any(|child| inner(child, name, values, context, active)),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty()
                    && children
                        .iter()
                        .all(|child| inner(child, name, values, context, active))
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, values, context, active)
                        && inner(else_schema, name, values, context, active)
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    inner(then_schema, name, values, context, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    inner(else_schema, name, values, context, active)
                }
                _ => false,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, name, values, context, &mut HashSet::new())
}

/// Return true when `finite_side` forces some present tuple item into a finite
/// value set, while `other` guarantees the same position and locally rejects
/// every value in that set. This mirrors the property discriminator helper for
/// tuple-tagged unions.
pub(super) fn finite_required_array_item_values_rejected_by_other(
    finite_side: &SchemaNode,
    other: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    const MAX_TRACKED_INDEX: u64 = 32;
    let Some(finite_len) = array_length_interval_bound(finite_side) else {
        return false;
    };
    let Some(other_len) = array_length_interval_bound(other) else {
        return false;
    };
    let shared = finite_len.lower.min(other_len.lower).min(MAX_TRACKED_INDEX);
    for index in 0..shared {
        let Ok(index_usize) = usize::try_from(index) else {
            break;
        };
        let Some(values) = finite_item_value_bound_at(finite_side, index_usize) else {
            continue;
        };
        if values.is_empty()
            || array_item_value_set_definitely_rejected(other, index_usize, &values, context)
        {
            return true;
        }
    }
    false
}

pub(super) fn finite_item_value_bound_at(schema: &SchemaNode, index: usize) -> Option<Vec<Value>> {
    fn dedup_extend(out: &mut Vec<Value>, values: Vec<Value>) {
        for value in values {
            if !out.iter().any(|seen| json_values_equal(seen, &value)) {
                out.push(value);
            }
        }
    }

    fn inner(
        schema: &SchemaNode,
        index: usize,
        active: &mut HashSet<NodeId>,
    ) -> Option<Vec<Value>> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(Vec::new()),
            SchemaNodeKind::Array {
                prefix_items,
                items,
                ..
            } => {
                let item_schema = prefix_items.get(index).unwrap_or(items);
                finite_schema_value_superset(item_schema)
            }
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .find_map(|child| inner(child, index, active)),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                if children.is_empty() {
                    None
                } else {
                    let mut out = Vec::new();
                    let mut complete = true;
                    for child in children {
                        match inner(child, index, active) {
                            Some(values) => dedup_extend(&mut out, values),
                            None => {
                                complete = false;
                                break;
                            }
                        }
                    }
                    complete.then_some(out)
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    // Regardless of which side of the guard an array takes,
                    // its item must be covered by one of the two branch
                    // bounds. This is a union upper bound for discriminator values.
                    match (
                        inner(then_schema, index, active),
                        inner(else_schema, index, active),
                    ) {
                        (Some(mut then_values), Some(else_values)) => {
                            dedup_extend(&mut then_values, else_values);
                            Some(then_values)
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    inner(then_schema, index, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    inner(else_schema, index, active)
                }
                _ => None,
            },
            SchemaNodeKind::Const(value) => value
                .as_array()
                .and_then(|array| array.get(index).cloned())
                .map(|value| vec![value]),
            SchemaNodeKind::Enum(values) => {
                let mut out = Vec::new();
                let mut complete = !values.is_empty();
                for value in values {
                    let Some(array) = value.as_array() else {
                        complete = false;
                        break;
                    };
                    let Some(item) = array.get(index) else {
                        complete = false;
                        break;
                    };
                    if !out.iter().any(|seen| json_values_equal(seen, item)) {
                        out.push(item.clone());
                    }
                }
                complete.then_some(out)
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, index, &mut HashSet::new())
}

pub(super) fn array_item_value_set_definitely_rejected(
    schema: &SchemaNode,
    index: usize,
    values: &[Value],
    context: &mut SubschemaCheckContext,
) -> bool {
    fn all_values_rejected(
        constraint: &SchemaNode,
        values: &[Value],
        context: &mut SubschemaCheckContext,
    ) -> bool {
        values
            .iter()
            .all(|value| context.schema_definitely_rejects_value(constraint, value))
    }

    fn inner(
        schema: &SchemaNode,
        index: usize,
        values: &[Value],
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<NodeId>,
    ) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) => false,
            SchemaNodeKind::Array {
                prefix_items,
                items,
                ..
            } => {
                let item_schema = prefix_items.get(index).unwrap_or(items);
                all_values_rejected(item_schema, values, context)
            }
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .any(|child| inner(child, index, values, context, active)),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty()
                    && children
                        .iter()
                        .all(|child| inner(child, index, values, context, active))
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, index, values, context, active)
                        && inner(else_schema, index, values, context, active)
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    inner(then_schema, index, values, context, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    inner(else_schema, index, values, context, active)
                }
                _ => false,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, index, values, context, &mut HashSet::new())
}

/// Return true when every value admitted by `schema` is known not to satisfy
/// `excluded`.  This is a small complement-aware helper for condition guards:
/// generated schemas often spell an else-only branch as `allOf: [{not: G}, ...]`.
/// We only use explicit `not` wrappers (plus applicator structure), and prove
/// the guard is contained by the negated schema with the regular subset
/// checker, so a `true` result is a sound disjointness fact.
pub(super) fn schema_definitely_excludes_schema(
    schema: &SchemaNode,
    excluded: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn inner(
        schema: &SchemaNode,
        excluded: &SchemaNode,
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<(NodeId, NodeId)>,
    ) -> bool {
        if !active.insert((schema.id(), excluded.id())) {
            return false;
        }

        use SchemaNodeKind::*;
        let result = match schema.kind() {
            BoolSchema(false) => true,
            AllOf(children) => children
                .iter()
                .any(|child| inner(child, excluded, context, active)),
            AnyOf(children) | OneOf(children) => children
                .iter()
                .all(|child| inner(child, excluded, context, active)),
            IfThenElse {
                then_schema,
                else_schema,
                ..
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, excluded, context, active)
                        && inner(else_schema, excluded, context, active)
                }
                // A missing conditional branch is the implicit `true` schema,
                // so it cannot by itself exclude anything.
                _ => false,
            },
            Not(negated) => {
                analyze_subschema_with_context(
                    excluded,
                    negated,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
            }
            _ => false,
        };

        active.remove(&(schema.id(), excluded.id()));
        result
    }

    inner(schema, excluded, context, &mut HashSet::new())
}

/// Return true when two schemas cannot share a value, using only cheap facts
/// needed by the `oneOf` partition proof. Type separation handles primitives;
/// for object-only overlap, also recognize the common discriminator pattern of
/// a required property constrained to disjoint finite value sets.
pub(super) fn schemas_definitely_disjoint_for_oneof(
    sub: &SchemaNode,
    sub_mask: u8,
    other: &SchemaNode,
) -> bool {
    let other_mask = possible_json_type_mask(other);
    let overlap = sub_mask & other_mask;
    if overlap == 0 {
        return true;
    }
    if overlap == JSON_TYPE_NUMBER
        && (numeric_intervals_are_disjoint(sub, other) || integer_lattices_are_disjoint(sub, other))
    {
        return true;
    }
    if overlap == JSON_TYPE_STRING && string_length_intervals_are_disjoint(sub, other) {
        return true;
    }
    if overlap == JSON_TYPE_ARRAY && array_length_intervals_are_disjoint(sub, other) {
        return true;
    }
    if overlap == JSON_TYPE_ARRAY && required_array_item_shapes_are_disjoint(sub, other) {
        return true;
    }
    if overlap == JSON_TYPE_OBJECT && object_property_count_intervals_are_disjoint(sub, other) {
        return true;
    }
    if overlap & !JSON_TYPE_OBJECT != 0 {
        return false;
    }
    required_property_values_are_disjoint(sub, other)
}

/// Cheap, sound disjointness facts that are useful outside the `oneOf`
/// partition proof as well (for example, tuple positions whose domains make
/// `uniqueItems` redundant). This intentionally exposes only the boolean
/// fact, not the type-mask machinery, so callers cannot accidentally rely on
/// the masks as exact languages.
pub(super) fn schemas_definitely_disjoint_by_shape(left: &SchemaNode, right: &SchemaNode) -> bool {
    let left_mask = possible_json_type_mask(left);
    left_mask == 0 || schemas_definitely_disjoint_for_oneof(left, left_mask, right)
}
