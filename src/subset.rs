use crate::SchemaNode;
use json_schema_ast::{ArrayContains, JSONSchema, SchemaNodeId, SchemaNodeKind, compile};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RecursiveValidationFrame {
    schema_id: SchemaNodeId,
    value_address: usize,
}

#[derive(Default)]
struct SubschemaCheckContext {
    compiled_sup_validators: HashMap<SchemaNodeId, Option<Rc<JSONSchema>>>,
}

impl SubschemaCheckContext {
    fn compile_sup_validator(&mut self, sup: &SchemaNode) -> Option<Rc<JSONSchema>> {
        let node_id = sup.id();
        if let Some(validator) = self.compiled_sup_validators.get(&node_id) {
            return validator.clone();
        }

        let validator = if schema_contains_cycle(sup, &mut Vec::new()) {
            None
        } else {
            compile(&sup.to_json()).ok().map(Rc::new)
        };
        self.compiled_sup_validators
            .insert(node_id, validator.clone());
        validator
    }

    fn superset_contains_value(&mut self, sup: &SchemaNode, value: &Value) -> bool {
        self.superset_contains_value_inner(sup, value, &mut HashSet::new())
    }

    fn superset_contains_value_set(&mut self, sup: &SchemaNode, values: &[Value]) -> bool {
        values
            .iter()
            .all(|value| self.superset_contains_value(sup, value))
    }

    fn superset_contains_value_inner(
        &mut self,
        sup: &SchemaNode,
        value: &Value,
        active: &mut HashSet<RecursiveValidationFrame>,
    ) -> bool {
        if let Some(validator) = self.compile_sup_validator(sup) {
            return validator.is_valid(value);
        }

        let frame = RecursiveValidationFrame {
            schema_id: sup.id(),
            value_address: std::ptr::from_ref(value) as usize,
        };
        if !active.insert(frame) {
            // Re-entering the exact same schema node on the exact same JSON value
            // is a non-productive cycle (`A = anyOf(integer, A)` on `"x"`). Fail
            // closed here; productive recursion over object/array children uses
            // distinct `Value` addresses and therefore continues to descend.
            return false;
        }

        let is_valid = match sup.kind() {
            SchemaNodeKind::BoolSchema(valid) => *valid,
            SchemaNodeKind::Any => true,
            SchemaNodeKind::String {
                min_length,
                max_length,
                enumeration,
                ..
            } => value.as_str().is_some_and(|value| {
                string_length_in_range(value, *min_length, *max_length)
                    && enum_contains_value(enumeration.as_deref(), &Value::String(value.to_owned()))
            }),
            SchemaNodeKind::Number {
                minimum,
                maximum,
                exclusive_minimum,
                exclusive_maximum,
                multiple_of,
                enumeration,
            } => value.as_f64().is_some_and(|value_number| {
                check_numeric_value(
                    value_number,
                    *minimum,
                    *exclusive_minimum,
                    *maximum,
                    *exclusive_maximum,
                ) && value_is_multiple_of(value_number, *multiple_of)
                    && enum_contains_numeric_value(enumeration.as_deref(), value_number)
            }),
            SchemaNodeKind::Integer {
                minimum,
                maximum,
                exclusive_minimum,
                exclusive_maximum,
                multiple_of,
                enumeration,
            } => value.as_f64().is_some_and(|value_number| {
                value_number.fract() == 0.0
                    && check_numeric_value(
                        value_number,
                        minimum.map(|bound| bound as f64),
                        *exclusive_minimum,
                        maximum.map(|bound| bound as f64),
                        *exclusive_maximum,
                    )
                    && value_is_multiple_of(value_number, *multiple_of)
                    && enum_contains_numeric_value(enumeration.as_deref(), value_number)
            }),
            SchemaNodeKind::Boolean { enumeration } => value
                .as_bool()
                .is_some_and(|_| enum_contains_value(enumeration.as_deref(), value)),
            SchemaNodeKind::Null { enumeration } => {
                value.is_null() && enum_contains_value(enumeration.as_deref(), value)
            }
            SchemaNodeKind::Object {
                properties,
                required,
                additional,
                property_names,
                min_properties,
                max_properties,
                dependent_required,
                enumeration,
            } => value.as_object().is_some_and(|value_object| {
                enum_contains_value(enumeration.as_deref(), value)
                    && min_properties.is_none_or(|minimum| value_object.len() >= minimum)
                    && max_properties.is_none_or(|maximum| value_object.len() <= maximum)
                    && required.iter().all(|name| value_object.contains_key(name))
                    && dependent_required.iter().all(|(trigger, dependencies)| {
                        !value_object.contains_key(trigger)
                            || dependencies
                                .iter()
                                .all(|dependency| value_object.contains_key(dependency))
                    })
                    && value_object.iter().all(|(name, property_value)| {
                        let property_name = Value::String(name.clone());
                        self.superset_contains_value_inner(property_names, &property_name, active)
                            && self.superset_contains_value_inner(
                                properties.get(name).unwrap_or(additional),
                                property_value,
                                active,
                            )
                    })
            }),
            SchemaNodeKind::Array {
                prefix_items,
                items,
                min_items,
                max_items,
                contains,
                unique_items,
                enumeration,
            } => value.as_array().is_some_and(|value_array| {
                enum_contains_value(enumeration.as_deref(), value)
                    && min_items.is_none_or(|minimum| value_array.len() as u64 >= minimum)
                    && max_items.is_none_or(|maximum| value_array.len() as u64 <= maximum)
                    && (!unique_items || array_values_are_unique(value_array))
                    && value_array.iter().enumerate().all(|(index, item)| {
                        let item_schema = prefix_items.get(index).unwrap_or(items);
                        self.superset_contains_value_inner(item_schema, item, active)
                    })
                    && contains.as_ref().is_none_or(|contains| {
                        let matching_items = value_array
                            .iter()
                            .filter(|item| {
                                self.superset_contains_value_inner(&contains.schema, item, active)
                            })
                            .count() as u64;
                        matching_items >= contains.min_contains
                            && contains
                                .max_contains
                                .is_none_or(|maximum| matching_items <= maximum)
                    })
            }),
            SchemaNodeKind::Defs(_) => true,
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .all(|child| self.superset_contains_value_inner(child, value, active)),
            SchemaNodeKind::AnyOf(children) => children
                .iter()
                .any(|child| self.superset_contains_value_inner(child, value, active)),
            SchemaNodeKind::OneOf(children) => {
                children
                    .iter()
                    .filter(|child| self.superset_contains_value_inner(child, value, active))
                    .count()
                    == 1
            }
            SchemaNodeKind::Not(child) => !self.superset_contains_value_inner(child, value, active),
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                if self.superset_contains_value_inner(if_schema, value, active) {
                    then_schema.as_ref().is_none_or(|then_schema| {
                        self.superset_contains_value_inner(then_schema, value, active)
                    })
                } else {
                    else_schema.as_ref().is_none_or(|else_schema| {
                        self.superset_contains_value_inner(else_schema, value, active)
                    })
                }
            }
            SchemaNodeKind::Const(expected) => value == expected,
            SchemaNodeKind::Enum(values) => values.contains(value),
            SchemaNodeKind::Type(_)
            | SchemaNodeKind::Minimum(_)
            | SchemaNodeKind::Maximum(_)
            | SchemaNodeKind::Required(_)
            | SchemaNodeKind::AdditionalProperties(_)
            | SchemaNodeKind::Format(_)
            | SchemaNodeKind::ContentEncoding(_)
            | SchemaNodeKind::ContentMediaType(_)
            | SchemaNodeKind::Title(_)
            | SchemaNodeKind::Description(_)
            | SchemaNodeKind::Default(_)
            | SchemaNodeKind::Examples(_)
            | SchemaNodeKind::ReadOnly(_)
            | SchemaNodeKind::WriteOnly(_)
            | SchemaNodeKind::Ref(_)
            | _ => false,
        };

        active.remove(&frame);
        is_valid
    }
}

/// Returns `true` if **every** instance that satisfies `sub` also satisfies
/// `sup`.
pub fn is_subschema_of(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    is_subschema_of_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

fn is_subschema_of_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if sub == sup {
        return true;
    }

    use SchemaNodeKind::*;

    let sub_kind = sub.kind().clone();
    let sup_kind = sup.kind().clone();

    match (&sub_kind, &sup_kind) {
        (BoolSchema(false), _) => true,
        (_, BoolSchema(true)) => true,
        (Any, Any) => true,
        (_, Any) => true,
        (Any, _) => false,

        // Keep sub-combinator handlers before sup-combinator handlers so when
        // both sides are unions we reason branch-wise on `sub` first.
        (AnyOf(subs), _) | (OneOf(subs), _) => subs
            .iter()
            .all(|branch| is_subschema_of_with_context(branch, sup, context)),
        (AllOf(subs), _) => subs
            .iter()
            .all(|schema| is_subschema_of_with_context(schema, sup, context)),

        (Enum(sub_e), Enum(sup_e)) => sub_e.iter().all(|v| sup_e.contains(v)),
        (Enum(sub_e), _) => context.superset_contains_value_set(sup, sub_e),

        (Const(s_val), Const(p_val)) => s_val == p_val,
        (Const(s_val), _) => context.superset_contains_value(sup, s_val),

        (_, AnyOf(sups)) | (_, OneOf(sups)) => sups
            .iter()
            .any(|branch| is_subschema_of_with_context(sub, branch, context)),
        (_, AllOf(sups)) => sups
            .iter()
            .all(|schema| is_subschema_of_with_context(sub, schema, context)),

        (_, Enum(_)) => false,

        (Not(subn), _) => match subn.kind() {
            Any | BoolSchema(true) => true,
            BoolSchema(false) => !matches!(sup_kind, Any | BoolSchema(true)),
            _ => false,
        },
        (_, Not(supn)) => match supn.kind() {
            Any | BoolSchema(true) => matches!(sub_kind, BoolSchema(false)),
            BoolSchema(false) => matches!(sub_kind, BoolSchema(true) | Any),
            _ => false,
        },

        (String { .. }, String { .. })
        | (Number { .. }, Number { .. })
        | (Integer { .. }, Integer { .. })
        | (Boolean { .. }, Boolean { .. })
        | (Null { .. }, Null { .. })
        | (Object { .. }, Object { .. })
        | (Array { .. }, Array { .. }) => type_constraints_subsumed_with_context(sub, sup, context),

        (Integer { .. }, Number { .. }) => integer_constraints_subsumed_by_number(sub, sup),

        (_, Const(_)) => false,

        _ => false,
    }
}

fn integer_constraints_subsumed_by_number(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    use SchemaNodeKind::*;

    let sub_kind = sub.kind().clone();
    let sup_kind = sup.kind().clone();

    let (
        Integer {
            minimum: smin,
            maximum: smax,
            exclusive_minimum: sexmin,
            exclusive_maximum: sexmax,
            multiple_of: smul,
            enumeration: s_en,
            ..
        },
        Number {
            minimum: pmin,
            maximum: pmax,
            exclusive_minimum: pexmin,
            exclusive_maximum: pexmax,
            multiple_of: pmul,
            enumeration: p_en,
            ..
        },
    ) = (sub_kind, sup_kind)
    else {
        return false;
    };

    if !check_numeric_inclusion(smin.map(|value| value as f64), sexmin, pmin, pexmin, true) {
        return false;
    }
    if !check_numeric_inclusion(smax.map(|value| value as f64), sexmax, pmax, pexmax, false) {
        return false;
    }
    if !check_multiple_of_inclusion(smul, pmul) {
        return false;
    }
    check_enum_inclusion(s_en.as_deref(), p_en.as_deref())
}

/// Compare the **constraints** of two nodes of the *same* basic type.
pub fn type_constraints_subsumed(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    type_constraints_subsumed_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

fn type_constraints_subsumed_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    use SchemaNodeKind::*;

    let sub_kind = sub.kind().clone();
    let sup_kind = sup.kind().clone();

    match (sub_kind, sup_kind) {
        (
            String {
                min_length: smin,
                max_length: smax,
                enumeration: s_enum,
                ..
            },
            String {
                min_length: pmin,
                max_length: pmax,
                enumeration: p_enum,
                ..
            },
        ) => {
            if let Some(pm) = pmin
                && smin.unwrap_or(0) < pm
            {
                return false;
            }
            if let Some(px) = pmax
                && smax.unwrap_or(u64::MAX) > px
            {
                return false;
            }
            if let (Some(se), Some(pe)) = (s_enum, p_enum)
                && !se.iter().all(|v| pe.contains(v))
            {
                return false;
            }
            true
        }

        (
            Number {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                multiple_of: smul,
                enumeration: s_en,
                ..
            },
            Number {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
                multiple_of: pmul,
                enumeration: p_en,
                ..
            },
        ) => {
            if !check_numeric_inclusion(smin, sexmin, pmin, pexmin, true) {
                return false;
            }
            if !check_numeric_inclusion(smax, sexmax, pmax, pexmax, false) {
                return false;
            }
            if !check_multiple_of_inclusion(smul, pmul) {
                return false;
            }
            check_enum_inclusion(s_en.as_deref(), p_en.as_deref())
        }

        (
            Integer {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                multiple_of: smul,
                enumeration: s_en,
                ..
            },
            Integer {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
                multiple_of: pmul,
                enumeration: p_en,
                ..
            },
        ) => {
            if !check_int_inclusion(smin, sexmin, pmin, pexmin, true) {
                return false;
            }
            if !check_int_inclusion(smax, sexmax, pmax, pexmax, false) {
                return false;
            }
            if !check_multiple_of_inclusion(smul, pmul) {
                return false;
            }
            check_enum_inclusion(s_en.as_deref(), p_en.as_deref())
        }

        (Boolean { enumeration: s_e }, Boolean { enumeration: p_e })
        | (Null { enumeration: s_e }, Null { enumeration: p_e }) => {
            check_enum_inclusion(s_e.as_deref(), p_e.as_deref())
        }

        (
            Object {
                properties: sprops,
                required: sreq,
                additional: s_addl,
                property_names: s_prop_names,
                min_properties: smin,
                max_properties: smax,
                dependent_required: _s_deps,
                enumeration: s_en,
            },
            Object {
                properties: pprops,
                required: preq,
                additional: p_addl,
                property_names: p_prop_names,
                min_properties: pmin,
                max_properties: pmax,
                dependent_required: p_deps,
                enumeration: p_en,
            },
        ) => {
            if let Some(pm) = pmin
                && smin.unwrap_or(0) < pm
            {
                return false;
            }
            if let Some(px) = pmax
                && smax.unwrap_or(usize::MAX) > px
            {
                return false;
            }

            if !check_enum_inclusion(s_en.as_deref(), p_en.as_deref()) {
                return false;
            }

            if !preq.is_subset(&sreq) {
                return false;
            }

            for (key, s_schema) in &sprops {
                if let Some(p_schema) = pprops.get(key) {
                    if !is_subschema_of_with_context(s_schema, p_schema, context) {
                        return false;
                    }
                } else {
                    // The new schema permits an additional property that the
                    // previous map did not list explicitly.  We must ensure the
                    // "additional" schema of the superset accepts whatever the
                    // subset would have produced (or, if `additionalProperties`
                    // was `false`, reject immediately).
                    let additional_allows =
                        !matches!(s_addl.kind(), SchemaNodeKind::BoolSchema(false));
                    if !additional_allows
                        || !is_subschema_of_with_context(s_schema, &p_addl, context)
                    {
                        return false;
                    }
                }
            }

            if !is_subschema_of_with_context(&s_addl, &p_addl, context) {
                return false;
            }

            if !is_subschema_of_with_context(&s_prop_names, &p_prop_names, context) {
                return false;
            }

            for (trigger, deps) in p_deps.iter() {
                // If the superset requires extra keys whenever `trigger` exists,
                // then the subset may only allow `trigger` when those keys are
                // unconditionally present.
                let trigger_allowed = sprops.contains_key(trigger)
                    || !matches!(s_addl.kind(), SchemaNodeKind::BoolSchema(false));
                if trigger_allowed && !deps.iter().all(|d| sreq.contains(d)) {
                    return false;
                }
            }

            true
        }

        (
            Array {
                prefix_items: s_prefix_items,
                items: sitems,
                min_items: smin,
                max_items: smax,
                contains: s_contains,
                unique_items: s_unique_items,
                enumeration: s_en,
            },
            Array {
                prefix_items: p_prefix_items,
                items: pitems,
                min_items: pmin,
                max_items: pmax,
                contains: p_contains,
                unique_items: p_unique_items,
                enumeration: p_en,
            },
        ) => {
            if let Some(pm) = pmin
                && smin.unwrap_or(0) < pm
            {
                return false;
            }
            if let Some(px) = pmax
                && smax.unwrap_or(u64::MAX) > px
            {
                return false;
            }
            if p_unique_items && !s_unique_items && smax.unwrap_or(u64::MAX) > 1 {
                return false;
            }
            if !array_item_constraints_subsumed(
                &s_prefix_items,
                &sitems,
                smax,
                &p_prefix_items,
                &pitems,
                context,
            ) {
                return false;
            }
            if !array_contains_constraints_subsumed(
                &s_prefix_items,
                &sitems,
                smin,
                smax,
                s_contains.as_ref(),
                p_contains.as_ref(),
                context,
            ) {
                return false;
            }
            if !check_enum_inclusion(s_en.as_deref(), p_en.as_deref()) {
                return false;
            }
            true
        }

        _ => false,
    }
}

fn check_numeric_inclusion(
    s_val: Option<f64>,
    s_excl: bool,
    p_val: Option<f64>,
    p_excl: bool,
    is_min: bool,
) -> bool {
    if p_val.is_none() {
        return true;
    }
    let supv = p_val.unwrap();
    let subv = s_val.unwrap_or(if is_min { f64::MIN } else { f64::MAX });

    if is_min {
        if p_excl {
            if s_excl { subv >= supv } else { subv > supv }
        } else {
            subv >= supv
        }
    } else if p_excl {
        if s_excl { subv <= supv } else { subv < supv }
    } else {
        subv <= supv
    }
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
    sub_min_items: Option<u64>,
    sub_max_items: Option<u64>,
    sub_contains: Option<&ArrayContains<SchemaNode>>,
    sup_contains: Option<&ArrayContains<SchemaNode>>,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(sup_contains) = sup_contains else {
        return true;
    };

    let lower_bound_ok = sup_contains.min_contains == 0
        || sub_contains.is_some_and(|sub_contains| {
            sub_contains.min_contains >= sup_contains.min_contains
                && is_subschema_of_with_context(&sub_contains.schema, &sup_contains.schema, context)
        })
        || (sub_min_items.unwrap_or(0) >= sup_contains.min_contains
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

    let Some(sup_max_contains) = sup_contains.max_contains else {
        return true;
    };

    sub_contains
        .filter(|sub_contains| {
            sub_contains
                .max_contains
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

fn array_values_are_unique(values: &[Value]) -> bool {
    values
        .iter()
        .enumerate()
        .all(|(index, value)| values[..index].iter().all(|seen| seen != value))
}

fn string_length_in_range(value: &str, minimum: Option<u64>, maximum: Option<u64>) -> bool {
    let length = value.chars().count() as u64;
    minimum.is_none_or(|minimum| length >= minimum)
        && maximum.is_none_or(|maximum| length <= maximum)
}

fn check_numeric_value(
    value: f64,
    minimum: Option<f64>,
    exclusive_minimum: bool,
    maximum: Option<f64>,
    exclusive_maximum: bool,
) -> bool {
    let above_minimum = match minimum {
        None => true,
        Some(minimum) if exclusive_minimum => value > minimum,
        Some(minimum) => value >= minimum,
    };
    let below_maximum = match maximum {
        None => true,
        Some(maximum) if exclusive_maximum => value < maximum,
        Some(maximum) => value <= maximum,
    };
    above_minimum && below_maximum
}

fn enum_contains_value(enumeration: Option<&[Value]>, value: &Value) -> bool {
    enumeration.is_none_or(|enumeration| enumeration.contains(value))
}

fn enum_contains_numeric_value(enumeration: Option<&[Value]>, value: f64) -> bool {
    enumeration.is_none_or(|enumeration| {
        enumeration
            .iter()
            .any(|expected| expected.as_f64().is_some_and(|expected| expected == value))
    })
}

fn check_multiple_of_inclusion(sub_multiple_of: Option<f64>, sup_multiple_of: Option<f64>) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };
    if sub_multiple_of <= 0.0 || sup_multiple_of <= 0.0 {
        return false;
    }

    if let (Some(sub_multiple_of), Some(sup_multiple_of)) = (
        exact_positive_integer(sub_multiple_of),
        exact_positive_integer(sup_multiple_of),
    ) {
        return sub_multiple_of % sup_multiple_of == 0;
    }

    let ratio = sub_multiple_of / sup_multiple_of;
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn value_is_multiple_of(value: f64, multiple_of: Option<f64>) -> bool {
    let Some(multiple_of) = multiple_of else {
        return true;
    };
    if multiple_of <= 0.0 {
        return false;
    }
    if let (Some(value), Some(multiple_of)) = (
        exact_positive_integer(value.abs()),
        exact_positive_integer(multiple_of),
    ) {
        return value % multiple_of == 0;
    }

    let ratio = value / multiple_of;
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn exact_positive_integer(value: f64) -> Option<u64> {
    if !value.is_finite() || value <= 0.0 || value.fract() != 0.0 || value > u64::MAX as f64 {
        return None;
    }

    let integer = value as u64;
    ((integer as f64) == value).then_some(integer)
}

fn check_enum_inclusion(sub_enum: Option<&[Value]>, sup_enum: Option<&[Value]>) -> bool {
    match (sub_enum, sup_enum) {
        (_, None) => true,
        (Some(sub_enum), Some(sup_enum)) => sub_enum.iter().all(|value| sup_enum.contains(value)),
        (None, Some(_)) => false,
    }
}

fn check_int_inclusion(
    s_val: Option<i64>,
    s_excl: bool,
    p_val: Option<i64>,
    p_excl: bool,
    is_min: bool,
) -> bool {
    if p_val.is_none() {
        return true;
    }
    let supv = p_val.unwrap();
    let subv = s_val.unwrap_or(if is_min { i64::MIN } else { i64::MAX });

    if is_min {
        if p_excl {
            if s_excl { subv >= supv } else { subv > supv }
        } else {
            subv >= supv
        }
    } else if p_excl {
        if s_excl { subv <= supv } else { subv < supv }
    } else {
        subv <= supv
    }
}

fn schema_contains_cycle(schema: &SchemaNode, path: &mut Vec<SchemaNode>) -> bool {
    if path.iter().any(|ancestor| ancestor.ptr_eq(schema)) {
        return true;
    }

    let children = schema_children(schema);
    if children.is_empty() {
        return false;
    }

    path.push(schema.clone());
    let contains_cycle = children
        .iter()
        .any(|child| schema_contains_cycle(child, path));
    path.pop();
    contains_cycle
}

fn schema_children(schema: &SchemaNode) -> Vec<SchemaNode> {
    use SchemaNodeKind::*;

    match schema.kind() {
        AllOf(children) | AnyOf(children) | OneOf(children) => children.clone(),
        Not(child) | AdditionalProperties(child) => vec![child.clone()],
        IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => std::iter::once(if_schema.clone())
            .chain(then_schema.iter().cloned())
            .chain(else_schema.iter().cloned())
            .collect(),
        Object {
            properties,
            additional,
            property_names,
            ..
        } => properties
            .values()
            .cloned()
            .chain(std::iter::once(additional.clone()))
            .chain(std::iter::once(property_names.clone()))
            .collect(),
        Array {
            prefix_items,
            items,
            contains,
            ..
        } => prefix_items
            .iter()
            .cloned()
            .chain(std::iter::once(items.clone()))
            .chain(contains.iter().map(|contains| contains.schema.clone()))
            .collect(),
        Defs(map) => map.values().cloned().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_schema_ast::build_and_resolve_schema;
    use serde_json::json;

    #[test]
    fn allof_tighten_subset() {
        let old = build_and_resolve_schema(&json!({
            "allOf": [
                {"type": "integer", "minimum": 0},
                {"maximum": 10}
            ]
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 5
        }))
        .unwrap();
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn exclusive_bounds_subset() {
        let old = build_and_resolve_schema(&json!({
            "minimum": 1,
            "exclusiveMinimum": 1
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
            "exclusiveMinimum": 1,
            "maximum": 3
        }))
        .unwrap();

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn enum_with_numeric_bound_is_not_subsumed_by_wider_enum() {
        let old = build_and_resolve_schema(&json!({
                "type": "number",
                "enum": [0, 1],
                "minimum": 1
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
                "enum": [0, 1]
        }))
        .unwrap();

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn integer_multiple_of_must_be_at_least_as_constrained_as_number_multiple_of() {
        let old = build_and_resolve_schema(&json!({
                "type": "number",
                "multipleOf": 2
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
                "type": "integer",
                "multipleOf": 3
        }))
        .unwrap();

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn huge_integer_multiple_of_ratio_must_be_exactly_divisible() {
        let old = build_and_resolve_schema(&json!({
                "type": "integer",
                "multipleOf": 3
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
                "type": "integer",
                "multipleOf": 9_007_199_254_740_994_f64
        }))
        .unwrap();

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn integer_schema_without_enum_is_not_subsumed_by_old_number_enum() {
        let old = build_and_resolve_schema(&json!({
                "type": "number",
                "enum": [1],
                "minimum": 1
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
                "type": "integer",
                "minimum": 1,
                "maximum": 2
        }))
        .unwrap();

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn enum_is_not_checked_by_serializing_recursive_sup_schema() {
        let sup = build_and_resolve_schema(&json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "next": { "$ref": "#/$defs/Node" }
                    }
                }
            },
            "$ref": "#/$defs/Node"
        }))
        .unwrap();
        let sub = build_and_resolve_schema(&json!({
            "enum": [1]
        }))
        .unwrap();

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn enum_values_are_checked_against_recursive_sup_schema_structurally() {
        let sup = build_and_resolve_schema(&json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "next": { "$ref": "#/$defs/Node" }
                    }
                }
            },
            "$ref": "#/$defs/Node"
        }))
        .unwrap();
        let sub = build_and_resolve_schema(&json!({
            "enum": [{ "next": {} }]
        }))
        .unwrap();

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn const_values_are_checked_against_recursive_sup_schema_structurally() {
        let sup = build_and_resolve_schema(&json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "value": { "type": "integer" },
                        "next": { "$ref": "#/$defs/Node" }
                    },
                    "required": ["value"]
                }
            },
            "$ref": "#/$defs/Node"
        }))
        .unwrap();
        let sub = build_and_resolve_schema(&json!({
            "const": {
                "value": 1,
                "next": {
                    "value": 2
                }
            }
        }))
        .unwrap();

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn const_values_reject_non_productive_recursive_sup_anyof() {
        let sup = build_and_resolve_schema(&json!({
            "$defs": {
                "A": {
                    "anyOf": [
                        { "type": "integer" },
                        { "$ref": "#/$defs/A" }
                    ]
                }
            },
            "$ref": "#/$defs/A"
        }))
        .unwrap();
        let sub = build_and_resolve_schema(&json!({
            "const": "x"
        }))
        .unwrap();

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn array_contains_lower_bound_must_hold_for_every_subset_instance() {
        let old = build_and_resolve_schema(&json!({
            "type": "array",
            "contains": { "type": "string" },
            "minContains": 2
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
            "type": "array",
            "items": { "type": "integer" },
            "minItems": 2
        }))
        .unwrap();

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn array_items_can_witness_contains_lower_bound_when_every_item_matches() {
        let old = build_and_resolve_schema(&json!({
            "type": "array",
            "contains": { "type": "string" },
            "minContains": 1
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
            "type": "array",
            "items": { "type": "string" },
            "minItems": 2
        }))
        .unwrap();

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn prefix_items_are_checked_positionally() {
        let old = build_and_resolve_schema(&json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }],
            "minItems": 1
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }],
            "minItems": 1
        }))
        .unwrap();

        assert!(!is_subschema_of(&new, &old));
    }
}
