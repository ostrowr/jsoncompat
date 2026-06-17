//! Conditional (`if`/`then`/`else`) subset helpers.
//!
//! This module keeps guard/branch reasoning together; callers still control
//! where these rules sit in the overall proof pipeline.

use super::*;

pub(super) fn optional_sup_conditional_branch_contains(
    sub: &SchemaNode,
    sup_branch: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    match sup_branch {
        None => true,
        Some(branch) => analyze_subschema_with_context(sub, branch, context, mode).is_subschema,
    }
}

pub(super) fn optional_conditional_branch_subsumed(
    sub_branch: Option<&SchemaNode>,
    sup_branch: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    match (sub_branch, sup_branch) {
        (_, None) => true,
        (Some(sub), Some(sup)) => {
            analyze_subschema_with_context(sub, sup, context, mode).is_subschema
        }
        (None, Some(sup)) => schema_is_trivially_universal(sup),
    }
}

pub(super) enum LiveConditionalBranch<'a> {
    /// The selected branch is absent, which is the JSON Schema `true` schema.
    Universal,
    Schema(&'a SchemaNode),
}

/// Return the only live branch for a conditional with a syntactically constant
/// guard.  Keep this deliberately narrow: recognizing literal/unconstrained
/// true and literal false is enough for common normalized schemas, while
/// avoiding general emptiness/complement reasoning in a normalization helper.
pub(super) fn constant_guard_conditional_branch<'a>(
    if_schema: &SchemaNode,
    then_branch: Option<&'a SchemaNode>,
    else_branch: Option<&'a SchemaNode>,
) -> Option<LiveConditionalBranch<'a>> {
    let selected = if schema_is_trivially_universal(if_schema) {
        then_branch
    } else if matches!(if_schema.kind(), SchemaNodeKind::BoolSchema(false)) {
        else_branch
    } else {
        return None;
    };

    Some(match selected {
        Some(branch) => LiveConditionalBranch::Schema(branch),
        None => LiveConditionalBranch::Universal,
    })
}

pub(super) fn explain_identical_conditional_failure(
    sub_then: Option<&SchemaNode>,
    sup_then: Option<&SchemaNode>,
    sub_else: Option<&SchemaNode>,
    sup_else: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    if !optional_conditional_branch_subsumed(
        sub_then,
        sup_then,
        context,
        ExplanationMode::VerdictOnly,
    ) {
        return explain_optional_conditional_branch_failure(sub_then, sup_then, context)
            .map(|detail| detail.under_conditional_branch("then"));
    }
    if !optional_conditional_branch_subsumed(
        sub_else,
        sup_else,
        context,
        ExplanationMode::VerdictOnly,
    ) {
        return explain_optional_conditional_branch_failure(sub_else, sup_else, context)
            .map(|detail| detail.under_conditional_branch("else"));
    }
    None
}

pub(super) fn explain_optional_conditional_branch_failure(
    sub_branch: Option<&SchemaNode>,
    sup_branch: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    match (sub_branch, sup_branch) {
        (Some(sub), Some(sup)) => explain_subschema_failure_with_context(sub, sup, context),
        (None, Some(_)) => Some(SubschemaExplanation::new(
            "unconstrained conditional branch is not contained by the comparison target branch",
        )),
        _ => None,
    }
}

/// Prove a finite conditional branch is contained by `sup` after restricting it
/// to the side of the guard on which the branch can actually run.  The finite
/// value list is an upper bound for the branch language; values that are
/// definitely rejected by the branch, definitely rejected by the guard (then
/// side), or definitely accepted by the guard (else side) can be ignored.
pub(super) fn finite_conditional_branch_values_fit_target(
    if_schema: &SchemaNode,
    branch: &SchemaNode,
    sup: &SchemaNode,
    then_side: bool,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(values) = finite_schema_value_superset(branch) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(branch, value) {
            return true;
        }
        let can_reach_side = if then_side {
            !context.schema_definitely_rejects_value(if_schema, value)
        } else {
            !context.superset_contains_value(if_schema, value)
        };
        !can_reach_side || context.superset_contains_value(sup, value)
    })
}

/// Dual finite proof for the `then` side: when the guard itself has a finite
/// value superset, enumerate that instead of the branch.  This catches cases
/// like `if: {enum:[1,"a"]}, then: {type: integer}` where the branch is not
/// finite on its own, but the reachable intersection is.  Values that the
/// guard or branch evaluator can soundly reject are ignored; any uncertain
/// value is kept, making the check conservative.
pub(super) fn finite_conditional_then_guard_values_fit_target(
    if_schema: &SchemaNode,
    then_branch: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(values) = finite_schema_value_superset(if_schema) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(if_schema, value)
            || context.schema_definitely_rejects_value(then_branch, value)
        {
            return true;
        }
        context.superset_contains_value(sup, value)
    })
}

/// Enumerate a finite guard domain to prove an implicit `then` branch fits a target.
/// This is the missing-branch analogue of `finite_conditional_then_guard_values_fit_target`.
pub(super) fn finite_guard_values_fit_target(
    if_schema: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(values) = finite_schema_value_superset(if_schema) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(if_schema, value) {
            return true;
        }
        context.superset_contains_value(sup, value)
    })
}

/// If the guard is `not A`, its else-side domain is contained in `A`.
/// Proving `A <= sup` is therefore enough for any else branch (including an
/// implicit/universal one) to fit the target.
pub(super) fn negated_guard_complement_subsumed_by_target(
    if_schema: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let SchemaNodeKind::Not(negated) = if_schema.kind() else {
        return false;
    };
    analyze_subschema_with_context(negated, sup, context, ExplanationMode::VerdictOnly).is_subschema
}

/// Finite fallback for an implicit/universal else side of a negated guard.
/// This is useful when `A <= sup` is too hard structurally but `A` has a small
/// enumerable superset.
pub(super) fn finite_negated_guard_complement_values_fit_target(
    if_schema: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let SchemaNodeKind::Not(negated) = if_schema.kind() else {
        return false;
    };
    let Some(values) = finite_schema_value_superset(negated) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(negated, value)
            || context.superset_contains_value(if_schema, value)
        {
            return true;
        }
        context.superset_contains_value(sup, value)
    })
}

/// Finite fallback for an explicit else branch under a negated guard; enumerate
/// the complement side (`A` in `not A`) and filter by the branch.
pub(super) fn finite_conditional_else_negated_guard_values_fit_target(
    if_schema: &SchemaNode,
    else_branch: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let SchemaNodeKind::Not(negated) = if_schema.kind() else {
        return false;
    };
    let Some(values) = finite_schema_value_superset(negated) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(negated, value)
            || context.schema_definitely_rejects_value(else_branch, value)
            || context.superset_contains_value(if_schema, value)
        {
            return true;
        }
        context.superset_contains_value(sup, value)
    })
}

pub(super) fn conditional_branches_subsumed_by(
    if_schema: &SchemaNode,
    then_branch: Option<&SchemaNode>,
    else_branch: Option<&SchemaNode>,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    fn then_branch_is_vacuous(
        if_schema: &SchemaNode,
        then_branch: &SchemaNode,
        context: &mut SubschemaCheckContext,
    ) -> bool {
        // The then side admits only values satisfying both the guard and the
        // branch.  Any sound disjointness fact for those two schemas makes the
        // side empty; false is merely conservative.
        let then_mask = possible_json_type_mask(then_branch);
        then_mask == 0
            || schemas_definitely_disjoint_for_partition(then_branch, then_mask, if_schema, context)
            || schema_definitely_excludes_schema(then_branch, if_schema, context)
            || schema_definitely_excludes_schema(if_schema, then_branch, context)
    }

    fn else_branch_is_vacuous(
        if_schema: &SchemaNode,
        else_branch: &SchemaNode,
        context: &mut SubschemaCheckContext,
    ) -> bool {
        // The else side admits only values that fail the guard.  If the branch
        // itself is a subset of the guard, no value can take that side.
        analyze_subschema_with_context(
            else_branch,
            if_schema,
            context,
            ExplanationMode::VerdictOnly,
        )
        .is_subschema
    }

    match (then_branch, else_branch) {
        (Some(then_branch), Some(else_branch)) => {
            (analyze_subschema_with_context(then_branch, sup, context, mode).is_subschema
                || then_branch_is_vacuous(if_schema, then_branch, context)
                || analyze_subschema_with_context(
                    if_schema,
                    sup,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
                || finite_conditional_branch_values_fit_target(
                    if_schema,
                    then_branch,
                    sup,
                    true,
                    context,
                )
                || finite_conditional_then_guard_values_fit_target(
                    if_schema,
                    then_branch,
                    sup,
                    context,
                ))
                && (analyze_subschema_with_context(else_branch, sup, context, mode).is_subschema
                    || else_branch_is_vacuous(if_schema, else_branch, context)
                    || branch_covers_guard_complement(if_schema, sup, context)
                    || negated_guard_complement_subsumed_by_target(if_schema, sup, context)
                    || finite_conditional_branch_values_fit_target(
                        if_schema,
                        else_branch,
                        sup,
                        false,
                        context,
                    )
                    || finite_conditional_else_negated_guard_values_fit_target(
                        if_schema,
                        else_branch,
                        sup,
                        context,
                    ))
        }
        (None, Some(else_branch)) => {
            // With no `then`, every value satisfying the guard is accepted on
            // that side.  It is therefore enough for the guard language itself
            // to be contained by the target, plus the usual proof for the
            // explicit else side.
            analyze_subschema_with_context(if_schema, sup, context, mode).is_subschema
                && (analyze_subschema_with_context(else_branch, sup, context, mode).is_subschema
                    || else_branch_is_vacuous(if_schema, else_branch, context)
                    || branch_covers_guard_complement(if_schema, sup, context)
                    || negated_guard_complement_subsumed_by_target(if_schema, sup, context)
                    || finite_conditional_branch_values_fit_target(
                        if_schema,
                        else_branch,
                        sup,
                        false,
                        context,
                    )
                    || finite_conditional_else_negated_guard_values_fit_target(
                        if_schema,
                        else_branch,
                        sup,
                        context,
                    ))
        }
        (Some(then_branch), None) => {
            // With no `else`, values failing the guard are unconstrained.  The
            // cheap sound special case is a universal guard, which leaves no
            // else side at all (or a universal target, as before).
            (schema_is_trivially_universal(sup)
                || schema_is_trivially_universal(if_schema)
                || branch_covers_guard_complement(if_schema, sup, context)
                || negated_guard_complement_subsumed_by_target(if_schema, sup, context)
                || finite_negated_guard_complement_values_fit_target(if_schema, sup, context))
                && (analyze_subschema_with_context(then_branch, sup, context, mode).is_subschema
                    || then_branch_is_vacuous(if_schema, then_branch, context)
                    || analyze_subschema_with_context(
                        if_schema,
                        sup,
                        context,
                        ExplanationMode::VerdictOnly,
                    )
                    .is_subschema
                    || finite_conditional_branch_values_fit_target(
                        if_schema,
                        then_branch,
                        sup,
                        true,
                        context,
                    )
                    || finite_conditional_then_guard_values_fit_target(
                        if_schema,
                        then_branch,
                        sup,
                        context,
                    ))
        }
        (None, None) => schema_is_trivially_universal(sup),
    }
}

pub(super) fn schemas_obviously_equivalent(a: &SchemaNode, b: &SchemaNode) -> bool {
    use SchemaNodeKind::*;
    match (a.kind(), b.kind()) {
        (Const(x), Const(y)) => json_values_equal(x, y),
        (Const(x), Enum(ys)) | (Enum(ys), Const(x)) => {
            ys.len() == 1 && ys.iter().any(|y| json_values_equal(x, y))
        }
        (Enum(xs), Enum(ys)) => {
            xs.len() == ys.len()
                && xs
                    .iter()
                    .all(|x| ys.iter().any(|y| json_values_equal(x, y)))
        }
        (BoolSchema(x), BoolSchema(y)) => x == y,
        (Any, Any) => true,
        (Null { enumeration: None }, Null { enumeration: None }) => true,
        (Boolean { enumeration: None }, Boolean { enumeration: None }) => true,
        (
            String {
                enumeration: None,
                length: al,
                pattern: ap,
                format: af,
            },
            String {
                enumeration: None,
                length: bl,
                pattern: bp,
                format: bf,
            },
        ) => al == bl && ap == bp && af == bf,
        (
            Number {
                enumeration: None,
                bounds: ab,
                multiple_of: am,
            },
            Number {
                enumeration: None,
                bounds: bb,
                multiple_of: bm,
            },
        ) => ab == bb && am == bm,
        (
            Integer {
                enumeration: None,
                bounds: ab,
                multiple_of: am,
            },
            Integer {
                enumeration: None,
                bounds: bb,
                multiple_of: bm,
            },
        ) => ab == bb && am == bm,
        _ => false,
    }
}

/// Compare two conditionals with an identical guard, taking account of
/// branches that are impossible on their guard side. Missing branches are the
/// JSON Schema `true` schema. For the guarded side, it is enough for the
/// superset branch to cover the guard; for the else side, a recognized
/// complement-cover branch can cover every value outside the guard.
pub(super) fn same_guard_conditional_branches_subsumed(
    guard: &SchemaNode,
    sub_then: Option<&SchemaNode>,
    sub_else: Option<&SchemaNode>,
    sup_then: Option<&SchemaNode>,
    sup_else: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    let then_empty = |branch: &SchemaNode, context: &mut SubschemaCheckContext| {
        let mask = possible_json_type_mask(branch);
        mask == 0
            || schemas_definitely_disjoint_for_partition(branch, mask, guard, context)
            || schema_definitely_excludes_schema(branch, guard, context)
            || schema_definitely_excludes_schema(guard, branch, context)
    };
    let else_empty = |branch: &SchemaNode, context: &mut SubschemaCheckContext| {
        analyze_subschema_with_context(branch, guard, context, ExplanationMode::VerdictOnly)
            .is_subschema
    };

    let then_ok = match (sub_then, sup_then) {
        (_, None) => true,
        (Some(sub_branch), Some(sup_branch)) => {
            then_empty(sub_branch, context)
                || analyze_subschema_with_context(sub_branch, sup_branch, context, mode)
                    .is_subschema
                || analyze_subschema_with_context(
                    guard,
                    sup_branch,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
                || finite_conditional_branch_values_fit_target(
                    guard, sub_branch, sup_branch, true, context,
                )
                || finite_conditional_then_guard_values_fit_target(
                    guard, sub_branch, sup_branch, context,
                )
        }
        (None, Some(sup_branch)) => {
            analyze_subschema_with_context(guard, sup_branch, context, ExplanationMode::VerdictOnly)
                .is_subschema
                || finite_guard_values_fit_target(guard, sup_branch, context)
        }
    };
    if !then_ok {
        return false;
    }

    match (sub_else, sup_else) {
        (_, None) => true,
        (Some(sub_branch), Some(sup_branch)) => {
            else_empty(sub_branch, context)
                || analyze_subschema_with_context(sub_branch, sup_branch, context, mode)
                    .is_subschema
                || branch_covers_guard_complement(guard, sup_branch, context)
                || finite_conditional_branch_values_fit_target(
                    guard, sub_branch, sup_branch, false, context,
                )
                || finite_conditional_else_negated_guard_values_fit_target(
                    guard, sub_branch, sup_branch, context,
                )
        }
        (None, Some(sup_branch)) => {
            branch_covers_guard_complement(guard, sup_branch, context)
                || negated_guard_complement_subsumed_by_target(guard, sup_branch, context)
                || finite_negated_guard_complement_values_fit_target(guard, sup_branch, context)
        }
    }
}

/// Return true for the small, sound class of conditionals that accept every
/// instance. Missing conditional branches are JSON Schema `true`; for an
/// explicit `then` branch, proving `if <= then` makes the guarded side
/// vacuous as a restriction. We deliberately only treat the else side as
/// covered when it is absent or syntactically universal, avoiding general
/// complement reasoning here.
pub(super) fn conditional_is_known_universal(
    if_schema: &SchemaNode,
    then_branch: Option<&SchemaNode>,
    else_branch: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
) -> bool {
    let then_covers_guard = match then_branch {
        None => true,
        Some(then_branch) => {
            schema_is_trivially_universal(then_branch)
                || analyze_subschema_with_context(
                    if_schema,
                    then_branch,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
        }
    };
    if !then_covers_guard {
        return false;
    }

    match else_branch {
        None => true,
        Some(else_branch) => branch_covers_guard_complement(if_schema, else_branch, context),
    }
}

/// Prove that a conditional else branch accepts every value outside `if_schema`.
/// A branch `not E` has that property whenever `E <= if_schema`: anything that
/// misses the guard cannot be in `E`, so it satisfies `not E`.  This also works
/// through an anyOf containing such a complement branch.  Keep the recognizer
/// deliberately small; failure is conservative.
pub(super) fn branch_covers_guard_complement(
    if_schema: &SchemaNode,
    branch: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if schema_is_trivially_universal(branch) {
        return true;
    }
    match branch.kind() {
        SchemaNodeKind::Not(excluded) => {
            analyze_subschema_with_context(
                excluded,
                if_schema,
                context,
                ExplanationMode::VerdictOnly,
            )
            .is_subschema
        }
        SchemaNodeKind::AnyOf(children) => children
            .iter()
            .any(|child| branch_covers_guard_complement(if_schema, child, context)),
        _ => false,
    }
}

pub(super) fn superset_conditional_contains(
    sub: &SchemaNode,
    sup: &SchemaNode,
    sup_if: &SchemaNode,
    sup_then: Option<&SchemaNode>,
    sup_else: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
) -> bool {
    let covered_by_then = optional_sup_conditional_branch_contains(
        sub,
        sup_then,
        context,
        ExplanationMode::VerdictOnly,
    );
    let covered_by_else = optional_sup_conditional_branch_contains(
        sub,
        sup_else,
        context,
        ExplanationMode::VerdictOnly,
    );
    let unconditional_branch_cover = covered_by_then && covered_by_else;

    let condition_always_true =
        analyze_subschema_with_context(sub, sup_if, context, ExplanationMode::VerdictOnly)
            .is_subschema;
    let sub_mask = possible_json_type_mask(sub);
    let condition_always_false = sub_mask == 0
        || schemas_definitely_disjoint_for_partition(sub, sub_mask, sup_if, context)
        || schema_definitely_excludes_schema(sub, sup_if, context);
    let guarded_branch_cover =
        (condition_always_true && covered_by_then) || (condition_always_false && covered_by_else);

    let branchwise_subset_cover = if let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    } = sub.kind()
    {
        conditional_branches_subsumed_by(
            if_schema,
            then_schema.as_ref(),
            else_schema.as_ref(),
            sup,
            context,
            ExplanationMode::VerdictOnly,
        )
    } else {
        false
    };

    unconditional_branch_cover || guarded_branch_cover || branchwise_subset_cover
}
