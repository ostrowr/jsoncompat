//! Array subset helpers.

use crate::SchemaNode;
use crate::subset::{SubschemaCheckContext, is_subschema_of_with_context};
use json_schema_ast::{ContainsConstraint, CountRange};
use serde_json::Value;

use super::scalar::check_enum_inclusion;

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
    sup.item_count.contains_range(sub.item_count)
        && (!sup.unique_items
            || sub.unique_items
            || sub.item_count.max().is_some_and(|max_items| max_items <= 1))
        && array_item_constraints_subsumed(
            sub.prefix_items,
            sub.items,
            sub.item_count.max(),
            sup.prefix_items,
            sup.items,
            context,
        )
        && array_contains_constraints_subsumed(
            sub.prefix_items,
            sub.items,
            sub.item_count,
            sub.contains,
            sup.contains,
            context,
        )
        && check_enum_inclusion(sub.enumeration, sup.enumeration)
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
        if !is_subschema_of_with_context(sub_item, sup_item, context) {
            return false;
        }
    }

    !array_index_can_exist(sub_max_items, checked_prefix_len)
        || is_subschema_of_with_context(sub_items, sup_items, context)
}

fn array_contains_constraints_subsumed(
    sub_prefix_items: &[SchemaNode],
    sub_items: &SchemaNode,
    sub_item_count: CountRange<u64>,
    sub_contains: Option<&ContainsConstraint<SchemaNode>>,
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
                && is_subschema_of_with_context(&sub_contains.schema, &sup_contains.schema, context)
        })
        || (sub_item_count.min() >= sup_contains_count.min()
            && all_array_item_schemas_subsumed_by(
                sub_prefix_items,
                sub_items,
                sub_max_items,
                &sup_contains.schema,
                context,
            ));

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
                && is_subschema_of_with_context(&sup_contains.schema, &sub_contains.schema, context)
        })
        .is_some()
        || (sub_max_items.is_some_and(|sub_max_items| sub_max_items <= sup_max_contains)
            && all_array_item_schemas_subsumed_by(
                sub_prefix_items,
                sub_items,
                sub_max_items,
                &sup_contains.schema,
                context,
            ))
}

fn all_array_item_schemas_subsumed_by(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    max_items: Option<u64>,
    sup_schema: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    for (index, prefix_item) in prefix_items.iter().enumerate() {
        if !array_index_can_exist(max_items, index) {
            return true;
        }
        if !is_subschema_of_with_context(prefix_item, sup_schema, context) {
            return false;
        }
    }

    !array_index_can_exist(max_items, prefix_items.len())
        || is_subschema_of_with_context(items, sup_schema, context)
}

fn array_index_can_exist(max_items: Option<u64>, index: usize) -> bool {
    let Ok(index) = u64::try_from(index) else {
        return false;
    };
    max_items.is_none_or(|max_items| index < max_items)
}
