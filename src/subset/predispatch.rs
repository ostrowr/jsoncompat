//! Pre-dispatch normalization and cover phases for subset analysis.
//!
//! These helpers run before the large structural `SchemaNodeKind` match. They
//! are kept in source order because many rules intentionally shadow more
//! general branch-wise reasoning.

use super::*;

/// Local contradictions that make the subset side empty before type dispatch.
pub(super) fn subset_is_locally_vacuous_before_dispatch(schema: &SchemaNode) -> bool {
    array_schema_is_locally_impossible(schema) || object_schema_is_locally_impossible(schema)
}

/// Normalize degenerate `oneOf` forms before general branch-wise reasoning.
///
/// This handles only exact/local-empty XOR identities; broader oneOf reasoning
/// stays in the dispatcher so it remains ordered with the other partition rules.
pub(super) fn try_normalize_trivial_one_of(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> Option<SubschemaAnalysis> {
    if let SchemaNodeKind::OneOf(branches) = sub.kind() {
        if one_of_pair_is_syntactically_empty(branches) {
            return Some(SubschemaAnalysis::compatible());
        }
        if let Some(live) = one_of_trivial_live_branch(branches) {
            return Some(if let Some(live) = live {
                analyze_subschema_with_context(live, sup, context, mode)
            } else {
                SubschemaAnalysis::compatible()
            });
        }
        if let Some(live) = one_of_pair_single_live_branch(branches) {
            return Some(analyze_subschema_with_context(live, sup, context, mode));
        }
    }
    if let SchemaNodeKind::OneOf(branches) = sup.kind() {
        if let Some(Some(live)) = one_of_trivial_live_branch(branches) {
            return Some(analyze_subschema_with_context(sub, live, context, mode));
        }
        // An empty oneOf on the right only contains an empty subset; leave that
        // to the ordinary machinery unless the subset was handled above.
        if let Some(live) = one_of_pair_single_live_branch(branches) {
            return Some(analyze_subschema_with_context(sub, live, context, mode));
        }
    }
    None
}

/// Peel exact wrapper identities (double negation and constant-guard conditionals).
pub(super) fn try_peel_exact_wrappers(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> Option<SubschemaAnalysis> {
    if let SchemaNodeKind::Not(outer) = sub.kind()
        && let SchemaNodeKind::Not(inner) = unwrap_singleton_applicators(outer).kind()
    {
        return Some(analyze_subschema_with_context(inner, sup, context, mode));
    }
    if let SchemaNodeKind::Not(outer) = sup.kind()
        && let SchemaNodeKind::Not(inner) = unwrap_singleton_applicators(outer).kind()
    {
        return Some(analyze_subschema_with_context(sub, inner, context, mode));
    }

    if let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    } = sup.kind()
        && let Some(live) =
            constant_guard_conditional_branch(if_schema, then_schema.as_ref(), else_schema.as_ref())
    {
        match live {
            LiveConditionalBranch::Universal => return Some(SubschemaAnalysis::compatible()),
            LiveConditionalBranch::Schema(branch) if branch.id() != sup.id() => {
                return Some(analyze_subschema_with_context(sub, branch, context, mode));
            }
            LiveConditionalBranch::Schema(_) => {}
        }
    }

    if let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    } = sub.kind()
        && let Some(live) =
            constant_guard_conditional_branch(if_schema, then_schema.as_ref(), else_schema.as_ref())
    {
        match live {
            LiveConditionalBranch::Universal => {
                let is_subschema = schema_is_trivially_universal(sup);
                return Some(SubschemaAnalysis::from_check(is_subschema, mode, || {
                    explain_schema_kind_gap(sub, sup)
                }));
            }
            LiveConditionalBranch::Schema(branch) if branch.id() != sub.id() => {
                return Some(analyze_subschema_with_context(branch, sup, context, mode));
            }
            LiveConditionalBranch::Schema(_) => {}
        }
    }

    None
}

/// Try pre-dispatch complement, partition, and cover identities that only prove success.
///
/// These rules are deliberately kept before the structural match: many of them
/// recognize normalized boolean encodings that would otherwise be treated as
/// ordinary branch-wise `oneOf`/`anyOf` relations.
pub(super) fn predispatch_cover_proves_subset(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    // The complement of a syntactically empty schema is universal.  This
    // catches common normalized contradictions such as `not(allOf[type:string,
    // type:array])` before the ordinary negation arm asks for contravariant
    // implication facts.
    if let SchemaNodeKind::Not(excluded) = sup.kind()
        && schema_is_locally_impossible_for_negation(unwrap_singleton_applicators(excluded))
    {
        return true;
    }

    // Push the same constant-guard conditional normalization through a
    // negated target.  `not(if true then A else B)` is just `not A`, so a
    // disjointness proof against the live branch is sufficient.  Do not
    // synthesize a temporary `Not` node; the existing negated-target
    // disjointness prover already has the right conservative structure.
    if let SchemaNodeKind::Not(excluded) = sup.kind()
        && let SchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } = unwrap_singleton_applicators(excluded).kind()
        && let Some(live) =
            constant_guard_conditional_branch(if_schema, then_schema.as_ref(), else_schema.as_ref())
    {
        match live {
            LiveConditionalBranch::Schema(branch)
                if schemas_definitely_disjoint_for_negation(sub, branch, context) =>
            {
                return true;
            }
            LiveConditionalBranch::Universal
                if schema_is_locally_empty_for_finite_enumeration(sub) =>
            {
                return true;
            }
            _ => {}
        }
    }

    // A conditional can be syntactically universal even when it is not the
    // literal `true` schema: if every value satisfying the guard is accepted
    // by the `then` branch, and the `else` branch is unconstrained (or absent),
    // the conditional accepts all instances. Recognize this before the main
    // structural match so such generated guards behave like a true superset.
    if let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    } = sup.kind()
        && conditional_is_known_universal(
            if_schema,
            then_schema.as_ref(),
            else_schema.as_ref(),
            context,
        )
    {
        return true;
    }

    // A union containing both `A` and `not B` is universal whenever B <= A.
    // This complement-cover pattern is common in generated partition schemas
    // and is safe to recognize before the ordinary branch-wise AnyOf logic.
    if let SchemaNodeKind::AnyOf(branches) = sup.kind()
        && (any_of_contains_known_universal_branch(branches, context)
            || any_of_complement_cover_is_universal(branches, context)
            || any_of_complement_union_cover_is_universal(branches, context)
            || any_of_property_presence_cover_is_universal(branches)
            || any_of_single_property_name_partition_is_universal(branches)
            || any_of_required_property_value_partition_is_universal(branches)
            || any_of_prefix_item_partition_is_universal(branches)
            || any_of_items_contains_partition_is_universal(branches)
            || any_of_integer_partition_cover_is_universal(branches)
            || any_of_numeric_range_cover_is_universal(branches)
            || any_of_string_length_cover_is_universal(branches)
            || any_of_array_count_cover_is_universal(branches)
            || any_of_object_count_cover_is_universal(branches))
    {
        return true;
    }

    // A two-arm `oneOf` of a schema and its complement is universal.
    if let SchemaNodeKind::OneOf(branches) = sup.kind()
        && one_of_complement_pair_is_universal(branches, context)
    {
        return true;
    }

    // A `oneOf` with one universal arm behaves like the complement of the
    // other arm.  Use local disjointness to prove a subset lands on exactly
    // the universal side.
    if let SchemaNodeKind::OneOf(branches) = sup.kind()
        && one_of_universal_arm_contains_subset(sub, branches, context)
    {
        return true;
    }

    // Likewise, `oneOf: [A, B, not(anyOf[A, B])]` is universal when the
    // positive siblings are known to be an exact disjoint cover of the union.
    if let SchemaNodeKind::OneOf(branches) = sup.kind()
        && one_of_complement_union_partition_is_universal(branches, context)
    {
        return true;
    }

    // The same finite complement partition can be spelled with `oneOf` when
    // the finite side is split into mutually exclusive sibling branches.
    if let SchemaNodeKind::OneOf(branches) = sup.kind()
        && one_of_finite_complement_partition_is_universal(branches, context)
    {
        return true;
    }

    // A two-arm xor of complements is contained in the union of its excluded
    // sides; prove each excluded side against the target directly.
    if let SchemaNodeKind::OneOf(branches) = sub.kind()
        && complement_only_oneof_subset_of(branches, sup, context)
    {
        return true;
    }

    // A two-arm xor with comparable positive arms is a set difference.
    // If every excluded target arm is either inside the removed side or
    // disjoint from the retained side, the difference fits the negation.
    if let SchemaNodeKind::OneOf(branches) = sub.kind()
        && let SchemaNodeKind::Not(excluded) = sup.kind()
        && comparable_oneof_difference_subset_of_negation(branches, excluded, context)
    {
        return true;
    }

    // A mixed xor `oneOf[A, not B]` collapses to `not(A ∪ B)` when A and B
    // are disjoint. In that case it is certainly contained by `not T` for
    // any T known to fit inside either excluded side. This catches compact
    // encodings of "everything except these two disjoint regions" without
    // constructing a synthetic union node.
    if let SchemaNodeKind::OneOf(branches) = sub.kind()
        && mixed_oneof_disjoint_complement_subset_of_target(branches, sup, context)
    {
        return true;
    }

    // Dual De Morgan shape: the complement of an intersection of explicit
    // complements is the union of their positive inners. If each positive
    // inner is contained by the target, the whole negated intersection is too.
    if let SchemaNodeKind::Not(excluded_intersection) = sub.kind()
        && let SchemaNodeKind::AllOf(conjuncts) =
            unwrap_singleton_applicators(excluded_intersection).kind()
        && conjuncts
            .iter()
            .all(|conjunct| matches!(conjunct.kind(), SchemaNodeKind::Not(_)))
        && conjuncts.iter().all(|conjunct| {
            if let SchemaNodeKind::Not(inner) = conjunct.kind() {
                analyze_subschema_with_context(inner, sup, context, ExplanationMode::VerdictOnly)
                    .is_subschema
            } else {
                false
            }
        })
    {
        return true;
    }

    // If the target mixed xor rejects only a finite comparable gap, it is
    // enough for a negated subset to exclude each gap value concretely.
    if let SchemaNodeKind::Not(excluded) = sub.kind()
        && negated_schema_excludes_mixed_finite_gap(
            unwrap_singleton_applicators(excluded),
            sup,
            context,
        )
    {
        return true;
    }

    // Complemented xor normalization for `not(oneOf[A, not B])` in the
    // conservative comparable/disjoint cases handled by the helper.
    if let SchemaNodeKind::Not(excluded_xor) = sub.kind()
        && let SchemaNodeKind::OneOf(children) = unwrap_singleton_applicators(excluded_xor).kind()
        && (negated_oneof_complement_pair_subset_of(children, sup, context)
            || negated_complement_pair_subset_of_mixed_difference(children, sup, context)
            || negated_xor_covers_mixed_comparable_gap(children, sup, context))
    {
        return true;
    }

    // For a two-arm xor, its complement accepts values that satisfy both
    // arms (or neither arm).  Prove either side directly before falling back
    // to more specialized complement-arm identities.
    if let SchemaNodeKind::Not(excluded_xor) = sup.kind()
        && let SchemaNodeKind::OneOf(children) = unwrap_singleton_applicators(excluded_xor).kind()
        && negated_two_arm_oneof_contains(sub, children, context)
    {
        return true;
    }

    // The complement of `oneOf[not A, B]` contains both A and B when
    // A and B are definitely disjoint: outside the overlap, the xor is false
    // exactly on those two regions.  This is a common spelling of a two-way
    // partition with one arm negated.
    if let SchemaNodeKind::Not(excluded_xor) = sup.kind()
        && let SchemaNodeKind::OneOf(children) = unwrap_singleton_applicators(excluded_xor).kind()
        && negated_oneof_complement_pair_contains(sub, children, context)
    {
        return true;
    }

    // Untyped array/object count assertions canonicalize the same way as
    // string lengths: all other JSON types plus one count half-line.
    if let SchemaNodeKind::Not(excluded_union) = sub.kind()
        && let SchemaNodeKind::AnyOf(children) = unwrap_singleton_applicators(excluded_union).kind()
        && (negated_untyped_array_count_halfline_subset_of(children, sup)
            || negated_untyped_object_count_halfline_subset_of(children, sup))
    {
        return true;
    }

    // Analogous canonicalization for untyped string length assertions:
    // all non-strings are accepted by the positive union, so its negation is
    // the opposite string-length half-line.
    if let SchemaNodeKind::Not(excluded_union) = sub.kind()
        && let SchemaNodeKind::AnyOf(children) = unwrap_singleton_applicators(excluded_union).kind()
        && negated_untyped_string_length_halfline_subset_of(children, sup)
    {
        return true;
    }

    // A negated canonicalized untyped numeric assertion has the shape
    // `not(anyOf[<all non-number types>, <numeric half-line>])`.  Once every
    // non-number type is covered, the complement is a numeric half-line with
    // the endpoint flipped; compare that interval directly to numeric targets.
    if let SchemaNodeKind::Not(excluded_union) = sub.kind()
        && let SchemaNodeKind::AnyOf(children) = unwrap_singleton_applicators(excluded_union).kind()
        && negated_untyped_numeric_halfline_subset_of(children, sup)
    {
        return true;
    }

    // One-sided De Morgan normalization: `not(anyOf[..., not A, ...])`
    // implies `A`. Do this before the main match because finite/enum targets
    // have early arms that would otherwise hide the negated-subset case.
    if let SchemaNodeKind::Not(excluded_union) = sub.kind()
        && let SchemaNodeKind::AnyOf(children) = unwrap_singleton_applicators(excluded_union).kind()
        && (negated_union_type_remainder_subset_of(children, sup)
            || negated_anyof_finite_complement_arm_subset_of(children, sup, context)
            || negated_union_subset_of_mixed_difference(children, sup, context)
            || children.iter().any(|child| {
                if let SchemaNodeKind::Not(required) = child.kind() {
                    analyze_subschema_with_context(
                        required,
                        sup,
                        context,
                        ExplanationMode::VerdictOnly,
                    )
                    .is_subschema
                } else {
                    false
                }
            }))
    {
        return true;
    }

    // A very common xor spelling for "objects without property p" is
    // `oneOf: [{type: object}, {type: object, required: [p]}]`.  The generic
    // oneOf rule below checks each arm independently, which is too strong: the
    // second arm is precisely removed by the xor. Recognize the narrow
    // presence-partition form before falling back to branch-wise reasoning.
    if let SchemaNodeKind::OneOf(children) = sub.kind()
        && (oneof_object_absence_partition_subset_of(children, sup)
            || oneof_array_empty_partition_subset_of(children, sup)
            || oneof_string_empty_partition_subset_of(children, sup)
            || oneof_object_empty_partition_subset_of(children, sup))
    {
        return true;
    }

    false
}
