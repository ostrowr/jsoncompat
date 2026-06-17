//! Array subset helpers.

use crate::SchemaNode;
use crate::subset::{SubschemaCheckContext, is_subschema_of_with_productive_context};
use json_schema_ast::{ContainsConstraint, CountRange};
use serde_json::Value;

use super::{
    finite_schema_value_superset, scalar::check_enum_inclusion,
    schema_definitely_rejects_all_values, schema_may_under_accept_values,
    schemas_definitely_disjoint_by_shape,
};

pub(super) struct ArrayConstraints<'a> {
    pub(super) prefix_items: &'a [SchemaNode],
    pub(super) items: &'a SchemaNode,
    pub(super) item_count: CountRange<u64>,
    pub(super) contains: Option<&'a ContainsConstraint<SchemaNode>>,
    pub(super) unique_items: bool,
    pub(super) enumeration: Option<&'a [Value]>,
}

pub(super) fn array_constraints_subsumed(
    sub: ArrayConstraints<'_>,
    sup: ArrayConstraints<'_>,
    context: &mut SubschemaCheckContext,
) -> bool {
    if array_constraints_definitely_uninhabited(&sub) {
        return true;
    }
    let Some(sub_item_count) = effective_item_count_for_unique_finite_domain(
        sub.prefix_items,
        sub.items,
        sub.item_count,
        sub.unique_items,
    ) else {
        // uniqueItems plus a finite item domain can make minItems impossible.
        return true;
    };
    if let Some(contains) = sub.contains
        && (contains_requirement_definitely_impossible(contains, sub_item_count, sub.unique_items)
            || contains_requirement_impossible_for_unique_finite_items(
                sub.prefix_items,
                sub.items,
                sub_item_count,
                contains,
                sub.unique_items,
            ))
    {
        // The inferred finite item-count/domain bounds can also make
        // minContains impossible, even when the contains schema itself has a
        // broad domain or is disjoint from the item domain.
        return true;
    }

    let sub_inherently_unique = sup.unique_items
        && !sub.unique_items
        && array_positions_guaranteed_unique(
            sub.prefix_items,
            sub.items,
            sub_item_count.max(),
            context,
        );

    sup.item_count.contains_range(sub_item_count)
        && (!sup.unique_items
            || sub.unique_items
            || sub_item_count.max().is_some_and(|max_items| max_items <= 1)
            || sub_inherently_unique)
        && array_item_constraints_subsumed(
            sub.prefix_items,
            sub.items,
            sub_item_count.max(),
            sup.prefix_items,
            sup.items,
            context,
        )
        && array_contains_constraints_subsumed(
            sub.prefix_items,
            sub.items,
            sub_item_count,
            sub.contains,
            sub.unique_items,
            sup.contains,
            context,
        )
        && check_enum_inclusion(sub.enumeration, sup.enumeration)
}

/// Prove that every array admitted by a non-`uniqueItems` subset is unique
/// anyway because each possible position has a disjoint value domain.
///
/// This intentionally handles only small, bounded tuple-like shapes.  A
/// homogeneous tail can be used at most once (unless it is definitely empty);
/// otherwise two tail positions could repeat the same value.  Pairwise
/// disjointness is proved from finite syntactic upper bounds, so unsupported
/// schemas simply make the shortcut give up.
fn array_positions_guaranteed_unique(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    max_items: Option<u64>,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(max_items) = max_items else {
        return false;
    };
    if max_items <= 1 {
        return true;
    }

    let prefix_len = u64::try_from(prefix_items.len()).unwrap_or(u64::MAX);
    if max_items > prefix_len {
        let tail_slots = max_items.saturating_sub(prefix_len);
        if tail_slots > 1 && !schema_definitely_rejects_all_values(items) {
            return false;
        }
    }

    let checked_prefix = prefix_items
        .len()
        .min(usize::try_from(max_items).unwrap_or(usize::MAX));
    // Keep the quadratic pairwise check cheap; larger generated tuples can
    // still be handled by explicit uniqueItems or other cardinality rules.
    if checked_prefix > 64 {
        return false;
    }

    for i in 0..checked_prefix {
        for j in (i + 1)..checked_prefix {
            if !schemas_definitely_disjoint(&prefix_items[i], &prefix_items[j], context) {
                return false;
            }
        }
    }

    // If exactly one tail position may occur, it must be disjoint from every
    // prefix position that can coexist with it.
    if max_items > prefix_len && !schema_definitely_rejects_all_values(items) {
        for prefix_item in &prefix_items[..checked_prefix] {
            if !schemas_definitely_disjoint(prefix_item, items, context) {
                return false;
            }
        }
    }

    true
}

fn schemas_definitely_disjoint(
    left: &SchemaNode,
    right: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if schema_definitely_rejects_all_values(left) || schema_definitely_rejects_all_values(right) {
        return true;
    }

    // Reuse the same cheap type/range/discriminator facts used for oneOf
    // partitioning. They are also sufficient to prove that two tuple slots can
    // never hold the same JSON value, which makes uniqueItems redundant for
    // common generated schemas like [string, number].
    if schemas_definitely_disjoint_by_shape(left, right) {
        return true;
    }

    if let Some(values) = finite_schema_value_superset(left)
        && values
            .iter()
            .all(|value| context.schema_definitely_rejects_value(right, value))
    {
        return true;
    }
    if let Some(values) = finite_schema_value_superset(right)
        && values
            .iter()
            .all(|value| context.schema_definitely_rejects_value(left, value))
    {
        return true;
    }
    false
}

/// Cheap, sound emptiness checks for array constraints.
///
/// If a position that must exist (because of `minItems`) has schema `false`,
/// no array can satisfy the subset branch. Likewise, if required tail
/// positions fall under `items: false`, the branch is empty.
pub(super) fn array_constraints_definitely_uninhabited(sub: &ArrayConstraints<'_>) -> bool {
    if let Some(contains) = sub.contains
        && contains_requirement_definitely_impossible(contains, sub.item_count, sub.unique_items)
    {
        return true;
    }

    // `uniqueItems` is a cross-position constraint. A common generated tuple
    // shape has repeated singleton/enum positions (for example two required
    // `const: "x"` entries). Per-position checks do not see the contradiction,
    // but if the union of finite domains for required positions is smaller
    // than the number of required positions, the array branch is empty.
    if unique_required_positions_exceed_finite_union(
        sub.prefix_items,
        sub.items,
        sub.item_count,
        sub.unique_items,
    ) {
        return true;
    }

    let min_items = sub.item_count.min();
    if min_items == 0 {
        return false;
    }

    let required_prefix_len = sub
        .prefix_items
        .len()
        .min(usize::try_from(min_items).unwrap_or(usize::MAX));
    if sub.prefix_items[..required_prefix_len]
        .iter()
        .any(schema_definitely_rejects_all_values)
    {
        return true;
    }

    min_items > u64::try_from(sub.prefix_items.len()).unwrap_or(u64::MAX)
        && schema_definitely_rejects_all_values(sub.items)
}

fn array_item_constraints_subsumed(
    sub_prefix_items: &[SchemaNode],
    sub_items: &SchemaNode,
    sub_max_items: Option<u64>,
    sup_prefix_items: &[SchemaNode],
    sup_items: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let checked_prefix_len = sub_prefix_items.len().max(sup_prefix_items.len());
    for index in 0..checked_prefix_len {
        if !array_index_can_exist(sub_max_items, index) {
            return true;
        }

        let sub_item = sub_prefix_items.get(index).unwrap_or(sub_items);
        let sup_item = sup_prefix_items.get(index).unwrap_or(sup_items);
        if !is_subschema_of_with_productive_context(sub_item, sup_item, context) {
            return false;
        }
    }

    !array_index_can_exist(sub_max_items, checked_prefix_len)
        || is_subschema_of_with_productive_context(sub_items, sup_items, context)
}

/// For homogeneous unique arrays over a finite item domain, pigeonhole the
/// non-matching values: if `minItems` exceeds the number of values that might
/// avoid `target`, some target matches are guaranteed. Values are counted as
/// matching only when the target membership check is known exact.
pub(super) fn unique_finite_domain_guarantees_contains_at_least(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    min_items: u64,
    unique_items: bool,
    target: &SchemaNode,
    required_matches: u64,
    context: &mut SubschemaCheckContext,
) -> bool {
    if required_matches == 0 {
        return true;
    }
    if !unique_items {
        return false;
    }

    let Some(values) = finite_domain_values(items) else {
        return false;
    };

    // Count prefix positions that must exist and are guaranteed to match the
    // target. Optional prefix positions cannot witness a lower bound.
    let guaranteed_prefix_len = prefix_items
        .len()
        .min(usize::try_from(min_items).unwrap_or(usize::MAX));
    let mut guaranteed_prefix_matches = 0_u64;
    let mut consumed_tail_nonmatches: Vec<Value> = Vec::new();
    for prefix_item in &prefix_items[..guaranteed_prefix_len] {
        if is_subschema_of_with_productive_context(prefix_item, target, context) {
            guaranteed_prefix_matches = guaranteed_prefix_matches.saturating_add(1);
            continue;
        }

        // A required singleton prefix value consumes that same value from the
        // unique tail domain. This matters for shapes like
        // prefixItems:[const "x"], items:enum["x","y"], minItems:2: the tail
        // can no longer use "x" to avoid a contains:{const "y"} requirement.
        if let Some(prefix_values) = finite_domain_values(prefix_item)
            && prefix_values.len() == 1
        {
            let value = &prefix_values[0];
            if context.schema_definitely_rejects_value(prefix_item, value) {
                // Required position has no inhabitant; the array branch is
                // empty, so any contains lower bound is vacuous.
                return true;
            }
            if context.superset_contains_value(target, value) {
                guaranteed_prefix_matches = guaranteed_prefix_matches.saturating_add(1);
                continue;
            }
            if context.schema_definitely_rejects_value(target, value) {
                push_distinct_owned(&mut consumed_tail_nonmatches, value);
            }
        }
    }
    if guaranteed_prefix_matches >= required_matches {
        return true;
    }

    // Only positions beyond prefixItems draw from the homogeneous finite tail
    // domain. With uniqueItems, at most one copy of each non-matching tail
    // value can be used, so enough required tail positions pigeonhole into
    // matching values. Ignoring overlap with prefix values is conservative: it
    // can only reduce the number of non-matching tail values still available.
    let prefix_capacity = u64::try_from(prefix_items.len()).unwrap_or(u64::MAX);
    let min_tail_items = min_items.saturating_sub(prefix_capacity);
    if min_tail_items == 0 {
        return false;
    }

    let domain_size = u64::try_from(values.len()).unwrap_or(u64::MAX);
    if min_tail_items > domain_size {
        // The subset branch is empty; callers may also detect this via the
        // effective item-count check, but treating it as vacuous here is sound.
        return true;
    }

    let definitely_matching = values
        .iter()
        .filter(|value| context.superset_contains_value(target, value))
        .count();
    let definitely_matching = u64::try_from(definitely_matching).unwrap_or(u64::MAX);
    let consumed_nonmatching = consumed_tail_nonmatches
        .iter()
        .filter(|consumed| {
            values.iter().any(|value| {
                json_schema_ast::json_values_equal(value, consumed)
                    && context.schema_definitely_rejects_value(target, value)
            })
        })
        .count();
    let consumed_nonmatching = u64::try_from(consumed_nonmatching).unwrap_or(u64::MAX);
    let maybe_nonmatching = domain_size
        .saturating_sub(definitely_matching)
        .saturating_sub(consumed_nonmatching);
    let forced_tail_matches = min_tail_items.saturating_sub(maybe_nonmatching);

    guaranteed_prefix_matches.saturating_add(forced_tail_matches) >= required_matches
}

pub(super) fn contains_requirement_definitely_impossible(
    contains: &ContainsConstraint<SchemaNode>,
    item_count: CountRange<u64>,
    unique_items: bool,
) -> bool {
    let required_matches = contains.count().min();
    required_matches > 0
        && (schema_definitely_rejects_all_values(&contains.schema)
            || item_count
                .max()
                .is_some_and(|max_items| required_matches > max_items)
            || (unique_items
                && finite_match_domain_size(&contains.schema)
                    .is_some_and(|domain_size| required_matches > domain_size)))
}

/// Return true when a unique array over a finite tail item domain cannot meet
/// its contains lower bound because too few positions could ever match.
///
/// Prefix positions are counted pessimistically as possible matches; this keeps
/// the check sound without solving schema intersections for each prefix item.
pub(super) fn contains_requirement_impossible_for_unique_finite_items(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    item_count: CountRange<u64>,
    contains: &ContainsConstraint<SchemaNode>,
    unique_items: bool,
) -> bool {
    let required_matches = contains.count().min();
    if required_matches == 0 || !unique_items {
        return false;
    }

    let Some(values) = finite_domain_values(items) else {
        return false;
    };

    let max_prefix_matches = u64::try_from(prefix_items.len()).unwrap_or(u64::MAX);
    let (max_prefix_matches, max_tail_slots) = match item_count.max() {
        Some(max_items) => {
            let prefix = max_prefix_matches.min(max_items);
            let prefix_len = u64::try_from(prefix_items.len()).unwrap_or(u64::MAX);
            (prefix, max_items.saturating_sub(prefix_len))
        }
        None => (max_prefix_matches, u64::MAX),
    };

    let may_under_accept = schema_may_under_accept_values(&contains.schema);
    let matching_tail_values = values
        .iter()
        .filter(|value| may_under_accept || contains.schema.accepts_value(value))
        .count();
    let matching_tail_values = u64::try_from(matching_tail_values).unwrap_or(u64::MAX);
    let max_tail_matches = matching_tail_values.min(max_tail_slots);

    required_matches > max_prefix_matches.saturating_add(max_tail_matches)
}

fn array_contains_constraints_subsumed(
    sub_prefix_items: &[SchemaNode],
    sub_items: &SchemaNode,
    sub_item_count: CountRange<u64>,
    sub_contains: Option<&ContainsConstraint<SchemaNode>>,
    sub_unique_items: bool,
    sup_contains: Option<&ContainsConstraint<SchemaNode>>,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(sup_contains) = sup_contains else {
        return true;
    };

    let sup_contains_count = sup_contains.count();
    let sub_max_items = sub_item_count.max();

    let lower_bound_ok = sup_contains_count.min() == 0
        || sub_contains.is_some_and(|sub_contains| {
            sub_contains.count().min() >= sup_contains_count.min()
                && is_subschema_of_with_productive_context(
                    &sub_contains.schema,
                    &sup_contains.schema,
                    context,
                )
        })
        || guaranteed_array_item_matches_at_least(
            sub_prefix_items,
            sub_items,
            sub_item_count.min(),
            &sup_contains.schema,
            sup_contains_count.min(),
            context,
        )
        || unique_finite_domain_guarantees_contains_at_least(
            sub_prefix_items,
            sub_items,
            sub_item_count.min(),
            sub_unique_items,
            &sup_contains.schema,
            sup_contains_count.min(),
            context,
        );

    if !lower_bound_ok {
        return false;
    }

    let Some(sup_max_contains) = sup_contains_count.max() else {
        return true;
    };

    sub_contains
        .filter(|sub_contains| {
            sub_contains
                .count()
                .max()
                .is_some_and(|sub_max_contains| sub_max_contains <= sup_max_contains)
                && is_subschema_of_with_productive_context(
                    &sup_contains.schema,
                    &sub_contains.schema,
                    context,
                )
        })
        .is_some()
        // A `maxContains` upper bound is automatically satisfied when the
        // subset array cannot have more total items than that bound.  Earlier
        // code also required every item schema to be a subset of the
        // `contains` schema, but that is stronger than necessary: the number
        // of matching items is always <= the total array length, regardless of
        // which items match.
        || sub_max_items.is_some_and(|sub_max_items| sub_max_items <= sup_max_contains)
        // With uniqueItems, an array can contain each JSON value at most once.
        // If the superset contains schema has a finite exact match domain, that
        // domain size is also an upper bound on matching items.
        || (sub_unique_items
            && finite_match_domain_size(&sup_contains.schema)
                .is_some_and(|domain_size| domain_size <= sup_max_contains))
        // Or, dually, the subset's item domains may be finite/disjoint enough
        // to bound how many positions can ever match the superset contains
        // schema. This is especially useful for `maxContains: 0` guards.
        || array_items_match_at_most(
            sub_prefix_items,
            sub_items,
            sub_item_count.max(),
            sub_unique_items,
            &sup_contains.schema,
            context,
        )
        .is_some_and(|max_matches| max_matches <= sup_max_contains)
}

pub(super) fn effective_item_count_for_unique_finite_domain(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    item_count: CountRange<u64>,
    unique_items: bool,
) -> Option<CountRange<u64>> {
    let mut max_items = item_count.max();

    // Literal `false` item schemas impose hard length ceilings: an array long
    // enough to reach that position would be invalid.  This is useful even
    // without uniqueItems (for example, optional `prefixItems: [false]`
    // effectively means maxItems=0).
    for (index, prefix_item) in prefix_items.iter().enumerate() {
        if schema_definitely_rejects_all_values(prefix_item) {
            let ceiling = u64::try_from(index).unwrap_or(u64::MAX);
            max_items = Some(max_items.map_or(ceiling, |max| max.min(ceiling)));
            break;
        }
    }
    if schema_definitely_rejects_all_values(items) {
        let ceiling = u64::try_from(prefix_items.len()).unwrap_or(u64::MAX);
        max_items = Some(max_items.map_or(ceiling, |max| max.min(ceiling)));
    }

    // A finite tail item domain and uniqueItems imply a finite array length
    // even when maxItems is absent.  With prefixItems, each prefix position can
    // contribute at most one value, and the homogeneous tail can contribute at
    // most one occurrence of each value in its finite domain.  This deliberately
    // uses the loose `prefix_len + tail_domain` bound rather than trying to
    // reason about overlaps between prefix and tail values.
    if unique_items && let Some(domain_size) = finite_match_domain_size(items) {
        let prefix_capacity = u64::try_from(prefix_items.len()).unwrap_or(u64::MAX);
        let inferred_max = prefix_capacity.saturating_add(domain_size);
        max_items = Some(max_items.map_or(inferred_max, |max| max.min(inferred_max)));
    }

    // Tighten the loose `prefix_len + tail_domain` bound when prefix domains
    // are finite too. Unknown prefix positions can still contribute at most
    // one distinct value each, while finite prefix/tail domains share a single
    // union of possible values.
    if unique_items && let Some(capacity) = finite_unique_position_capacity(prefix_items, items) {
        max_items = Some(max_items.map_or(capacity, |max| max.min(capacity)));
    }

    CountRange::new(item_count.min(), max_items)
}

pub(super) fn finite_match_domain_size(schema: &SchemaNode) -> Option<u64> {
    finite_domain_values(schema).and_then(|values| u64::try_from(values.len()).ok())
}

fn finite_domain_values(schema: &SchemaNode) -> Option<Vec<Value>> {
    finite_schema_value_superset(schema)
}

/// Conservative upper bound on how many items in any subset array can match
/// `target`. Returns `None` when an unbounded tail may still match.
pub(super) fn array_items_match_at_most(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    max_items: Option<u64>,
    unique_items: bool,
    target: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> Option<u64> {
    let possible_prefix_len = match max_items {
        Some(max_items) => prefix_items
            .len()
            .min(usize::try_from(max_items).unwrap_or(usize::MAX)),
        None => prefix_items.len(),
    };

    let mut bound = 0_u64;
    for prefix_item in &prefix_items[..possible_prefix_len] {
        if schemas_definitely_disjoint_by_shape(prefix_item, target)
            || finite_domain_values(prefix_item).is_some_and(|values| {
                values
                    .iter()
                    .all(|value| context.schema_definitely_rejects_value(target, value))
            })
        {
            continue;
        }
        bound = bound.saturating_add(1);
    }

    let prefix_len_u64 = u64::try_from(prefix_items.len()).unwrap_or(u64::MAX);
    let tail_slots = max_items.map(|max_items| max_items.saturating_sub(prefix_len_u64));
    if tail_slots == Some(0) {
        return Some(bound);
    }

    if schemas_definitely_disjoint_by_shape(items, target) {
        return Some(bound);
    }

    let Some(tail_values) = finite_domain_values(items) else {
        return tail_slots.map(|slots| bound.saturating_add(slots));
    };
    let maybe_matching_tail_values = tail_values
        .iter()
        .filter(|value| !context.schema_definitely_rejects_value(target, value))
        .count();
    let maybe_matching_tail_values = u64::try_from(maybe_matching_tail_values).unwrap_or(u64::MAX);

    if maybe_matching_tail_values == 0 {
        return Some(bound);
    }

    if !unique_items {
        return tail_slots.map(|slots| bound.saturating_add(slots));
    }

    // If a tail item exists, every prefix position exists too. Forced
    // singleton prefix values therefore consume the same JSON value from the
    // unique tail. This can tighten maxContains proofs for shapes like
    // prefixItems:[const "x"], items:enum["x", "y"], uniqueItems:true,
    // contains:{const "x"}: the tail cannot provide a second "x" match.
    // Keep the prefix-only bound as an alternative, since an array may choose
    // to stop before the homogeneous tail.
    let prefix_only_bound = bound;
    let mut consumed_matching_tail_values: Vec<Value> = Vec::new();
    let tail_can_exist = tail_slots.is_none_or(|slots| slots > 0);
    if tail_can_exist {
        for prefix_item in prefix_items {
            let Some(prefix_values) = finite_domain_values(prefix_item) else {
                continue;
            };
            if prefix_values.len() != 1 {
                continue;
            }
            let prefix_value = &prefix_values[0];
            if context.schema_definitely_rejects_value(target, prefix_value) {
                continue;
            }
            if tail_values.iter().any(|tail_value| {
                json_schema_ast::json_values_equal(tail_value, prefix_value)
                    && !context.schema_definitely_rejects_value(target, tail_value)
            }) {
                push_distinct_owned(&mut consumed_matching_tail_values, prefix_value);
            }
        }
    }
    let consumed_matching_tail_values =
        u64::try_from(consumed_matching_tail_values.len()).unwrap_or(u64::MAX);
    let available_matching_tail_values =
        maybe_matching_tail_values.saturating_sub(consumed_matching_tail_values);

    let with_tail_bound = match tail_slots {
        Some(slots) => bound.saturating_add(available_matching_tail_values.min(slots)),
        None => bound.saturating_add(available_matching_tail_values),
    };
    Some(prefix_only_bound.max(with_tail_bound))
}

fn push_distinct_owned(distinct: &mut Vec<Value>, value: &Value) {
    if !distinct
        .iter()
        .any(|seen| json_schema_ast::json_values_equal(seen, value))
    {
        distinct.push(value.clone());
    }
}

/// Upper bound on the number of distinct values available to a unique array
/// when the homogeneous tail has a finite domain. Prefix positions with an
/// unknown domain are counted pessimistically as one fresh value each.
fn finite_unique_position_capacity(prefix_items: &[SchemaNode], items: &SchemaNode) -> Option<u64> {
    let tail_values = finite_domain_values(items)?;
    let mut union = Vec::new();
    for value in tail_values {
        push_distinct_owned(&mut union, &value);
    }

    let mut unknown_prefix_positions = 0_u64;
    for prefix_item in prefix_items {
        if let Some(values) = finite_domain_values(prefix_item) {
            for value in values {
                push_distinct_owned(&mut union, &value);
            }
        } else {
            unknown_prefix_positions = unknown_prefix_positions.saturating_add(1);
        }
    }

    Some(
        u64::try_from(union.len())
            .unwrap_or(u64::MAX)
            .saturating_add(unknown_prefix_positions),
    )
}

/// Return true when required unique positions draw from too small a finite
/// union of possible values. Each finite domain is a superset of raw values, so
/// a pigeonhole failure remains sound even for conservatively evaluated enum
/// members.
fn unique_required_positions_exceed_finite_union(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    item_count: CountRange<u64>,
    unique_items: bool,
) -> bool {
    if !unique_items || item_count.min() <= 1 {
        return false;
    }

    let required = item_count.min();
    let required_prefix = prefix_items
        .len()
        .min(usize::try_from(required).unwrap_or(usize::MAX));
    let required_tail = required.saturating_sub(u64::try_from(required_prefix).unwrap_or(u64::MAX));

    // Collect a finite superset for every required position.  The old global
    // union pigeonhole check catches many contradictions, but it misses Hall
    // failures in a subset of positions (for example three required prefix
    // slots all limited to {1,2}, plus a fourth slot with many unrelated
    // values).  Since these are supersets, any Hall violation we find remains
    // sound for the real languages.
    let mut position_domains: Vec<Vec<Value>> = Vec::new();
    let mut global_union: Vec<Value> = Vec::new();

    for prefix_item in &prefix_items[..required_prefix] {
        let Some(values) = finite_domain_values(prefix_item) else {
            return false;
        };
        let domain = distinct_values(values);
        if domain.is_empty() {
            return true;
        }
        for value in &domain {
            push_distinct_owned(&mut global_union, value);
        }
        position_domains.push(domain);
    }

    if required_tail > 0 {
        let Some(values) = finite_domain_values(items) else {
            return false;
        };
        let tail_domain = distinct_values(values);
        if tail_domain.is_empty() {
            return true;
        }
        // Tail positions are homogeneous.  Even if prefix positions have many
        // extra values, more required tail slots than distinct tail values is
        // immediately impossible under uniqueItems.
        if required_tail > u64::try_from(tail_domain.len()).unwrap_or(u64::MAX) {
            return true;
        }
        for value in &tail_domain {
            push_distinct_owned(&mut global_union, value);
        }
        if let Ok(tail_count) = usize::try_from(required_tail) {
            // Keep the exact Hall scan tiny; the global checks below still
            // handle large generated tuples conservatively.
            if required_prefix.saturating_add(tail_count) <= 10 {
                for _ in 0..tail_count {
                    position_domains.push(tail_domain.clone());
                }
            }
        }
    }

    if u64::try_from(global_union.len()).unwrap_or(u64::MAX) < required {
        return true;
    }

    position_domains.len() == usize::try_from(required).unwrap_or(usize::MAX)
        && position_domains.len() <= 10
        && has_hall_violation(&position_domains)
}

fn distinct_values(values: Vec<Value>) -> Vec<Value> {
    let mut distinct = Vec::new();
    for value in values {
        push_distinct_owned(&mut distinct, &value);
    }
    distinct
}

fn has_hall_violation(domains: &[Vec<Value>]) -> bool {
    let n = domains.len();
    if n <= 1 || n > 10 {
        return false;
    }

    // Enumerate non-empty subsets of positions.  For each subset, Hall's
    // theorem requires at least as many distinct candidate values as slots.
    for mask in 1_usize..(1_usize << n) {
        let slots = mask.count_ones() as usize;
        if slots <= 1 {
            continue;
        }
        let mut union: Vec<Value> = Vec::new();
        for (index, domain) in domains.iter().enumerate() {
            if (mask & (1_usize << index)) == 0 {
                continue;
            }
            for value in domain {
                push_distinct_owned(&mut union, value);
                if union.len() >= slots {
                    break;
                }
            }
            if union.len() >= slots {
                break;
            }
        }
        if union.len() < slots {
            return true;
        }
    }
    false
}

fn guaranteed_array_item_matches_at_least(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    guaranteed_items: u64,
    sup_schema: &SchemaNode,
    required_matches: u64,
    context: &mut SubschemaCheckContext,
) -> bool {
    if required_matches == 0 {
        return true;
    }

    let guaranteed_prefix_items = prefix_items
        .len()
        .min(usize::try_from(guaranteed_items).unwrap_or(usize::MAX));
    let mut guaranteed_matches = 0_u64;
    for prefix_item in &prefix_items[..guaranteed_prefix_items] {
        if is_subschema_of_with_productive_context(prefix_item, sup_schema, context) {
            guaranteed_matches += 1;
            if guaranteed_matches >= required_matches {
                return true;
            }
        }
    }

    let guaranteed_tail_items =
        guaranteed_items.saturating_sub(u64::try_from(guaranteed_prefix_items).unwrap_or(u64::MAX));
    guaranteed_tail_items > 0
        && is_subschema_of_with_productive_context(items, sup_schema, context)
        && guaranteed_matches.saturating_add(guaranteed_tail_items) >= required_matches
}

fn array_index_can_exist(max_items: Option<u64>, index: usize) -> bool {
    let Ok(index) = u64::try_from(index) else {
        return false;
    };
    max_items.is_none_or(|max_items| index < max_items)
}
