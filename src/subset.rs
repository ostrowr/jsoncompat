//! Structural subset checks over the resolved schema IR.
//!
//! `is_subschema_of(sub, sup)` answers whether every instance accepted by
//! `sub` is also accepted by `sup`.  The checker is intentionally conservative
//! for hard cases such as regex implication and `oneOf` on the right-hand side.

use crate::SchemaNode;
use json_schema_ast::{
    CountRange, IntegerBounds, IntegerMultipleOf, NodeId, NumberBound, NumberBounds,
    NumberMultipleOf, PatternSupport, SchemaNodeKind, json_values_equal,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

// The public entry points live in this file; `dispatcher` owns the ordered
// recursive pipeline. Modules are grouped by proof responsibility rather than
// schema keyword: small, one-sided fact providers feed the dispatcher, while
// applicator-specific modules handle larger structural proof patterns.
mod analysis;
mod explanation;
mod membership;

// Keyword/type constraint checkers.
mod array;
mod object;
mod scalar;

// Conservative fact providers (all `false`/`None` results mean "unknown").
mod emptiness;
mod enumeration;
mod finite;
mod intervals;
mod properties;
mod type_masks;

// Higher-level proof phases and diagnostics.
mod boolean;
mod conditional;
mod disjoint;
mod dispatcher;
mod explainers;
mod partitions;
mod predispatch;

// Internal prelude: several proof modules use `super::*` so sibling helper
// functions remain reachable without making them crate-public. Keep new exports
// `pub(super)` and prefer explicit imports in small leaf modules.
use analysis::{ExplanationMode, SubschemaAnalysis};
use boolean::*;
use conditional::*;
use disjoint::*;
use dispatcher::*;
use emptiness::*;
use enumeration::*;
use explainers::*;
use explanation::SubschemaExplanation;
use finite::*;
use intervals::*;
use membership::{SubschemaCheckContext, schema_may_under_accept_values};
use partitions::*;
use predispatch::{
    predispatch_cover_proves_subset, subset_is_locally_vacuous_before_dispatch,
    try_normalize_trivial_one_of, try_peel_exact_wrappers,
};
use properties::*;

use object::dependent_requirement_is_guaranteed;
use type_masks::*;

use scalar::{
    StringConstraints, check_enum_inclusion, integer_constraints_subsumed_by_number,
    number_constraints_subsumed_by_integer, string_constraints_subsumed,
};

/// Returns `true` if **every** instance that satisfies `sub` also satisfies
/// `sup`.
pub(crate) fn is_subschema_of(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    is_subschema_of_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

pub(crate) fn explain_subschema_failure(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> Option<SubschemaExplanation> {
    explain_subschema_failure_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

pub(crate) fn explain_subschema_failure_emitted_values(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> Option<SubschemaExplanation> {
    explain_subschema_failure_with_context(
        sub,
        sup,
        &mut SubschemaCheckContext::for_emitted_values(),
    )
}

fn explain_subschema_failure_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    analyze_subschema_with_context(sub, sup, context, ExplanationMode::Explain).explanation
}

/// Variant of [`is_subschema_of`] that models serializer output rather than
/// full JSON Schema validity for the subset side.
///
/// In this mode, object schemas on the subset side are treated as if
/// undeclared extra properties are never emitted, even when
/// `additionalProperties` would permit them.
pub(crate) fn is_subschema_of_emitted_values(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    is_subschema_of_with_context(sub, sup, &mut SubschemaCheckContext::for_emitted_values())
}

pub(super) fn is_subschema_of_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    analyze_subschema_with_context(sub, sup, context, ExplanationMode::VerdictOnly).is_subschema
}

pub(super) fn is_subschema_of_with_productive_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    context.with_productive_frame(|context| is_subschema_of_with_context(sub, sup, context))
}

#[cfg(test)]
mod tests;
