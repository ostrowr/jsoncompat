//! Scalar subset helpers for numbers, integers, strings, booleans, and nulls.

use crate::SchemaNode;
use json_schema_ast::{
    CountRange, IntegerBounds, IntegerMultipleOf, NumberBound, NumberBounds, NumberMultipleOf,
    PatternConstraint, SchemaNodeKind, json_values_equal,
};
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub(super) struct StringConstraints<'a> {
    pub(super) length: CountRange<u64>,
    pub(super) pattern: Option<&'a PatternConstraint>,
    pub(super) enumeration: Option<&'a [Value]>,
}

pub(super) fn string_constraints_subsumed(
    sub: StringConstraints<'_>,
    sup: StringConstraints<'_>,
) -> bool {
    sup.length.contains_range(sub.length)
        && required_constraint_is_preserved(sub.pattern, sup.pattern)
        && check_enum_inclusion(sub.enumeration, sup.enumeration)
}

pub(super) fn number_constraints_subsumed(
    sub_bounds: NumberBounds,
    sub_multiple_of: Option<&NumberMultipleOf>,
    sub_enum: Option<&[Value]>,
    sup_bounds: NumberBounds,
    sup_multiple_of: Option<&NumberMultipleOf>,
    sup_enum: Option<&[Value]>,
) -> bool {
    sup_bounds.contains_bounds(sub_bounds)
        && check_multiple_of_inclusion(sub_multiple_of, sup_multiple_of)
        && check_enum_inclusion(sub_enum, sup_enum)
}

pub(super) fn integer_constraints_subsumed(
    sub_bounds: IntegerBounds,
    sub_multiple_of: Option<&IntegerMultipleOf>,
    sub_enum: Option<&[Value]>,
    sup_bounds: IntegerBounds,
    sup_multiple_of: Option<&IntegerMultipleOf>,
    sup_enum: Option<&[Value]>,
) -> bool {
    sup_bounds.contains_bounds(sub_bounds)
        && check_integer_multiple_of_inclusion(sub_multiple_of, sup_multiple_of)
        && check_enum_inclusion(sub_enum, sup_enum)
}

pub(super) fn integer_constraints_subsumed_by_number(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    let (
        SchemaNodeKind::Integer {
            bounds: sub_bounds,
            multiple_of: sub_multiple_of,
            enumeration: sub_enum,
        },
        SchemaNodeKind::Number {
            bounds: sup_bounds,
            multiple_of: sup_multiple_of,
            enumeration: sup_enum,
        },
    ) = (sub.kind(), sup.kind())
    else {
        return false;
    };

    sup_bounds.contains_bounds(sub_bounds.as_number_bounds())
        && check_integer_multiple_of_inclusion_by_number(
            sub_multiple_of.as_ref(),
            sup_multiple_of.as_ref(),
        )
        && check_enum_inclusion(sub_enum.as_deref(), sup_enum.as_deref())
}

/// Return true when a `number` schema is finitely pinned tightly enough to
/// check directly against an `integer` schema.
///
/// We intentionally do **not** infer integer-ness from `multipleOf: 1`: the
/// validator uses an epsilon tolerance for numeric multiples, so near-integer
/// floating values can satisfy such a number schema while failing `type:
/// integer`.  Only exact singleton bounds and finite enums are handled here.
pub(super) fn number_constraints_subsumed_by_integer(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    let (
        SchemaNodeKind::Number {
            bounds: sub_bounds,
            enumeration: sub_enum,
            ..
        },
        SchemaNodeKind::Integer { .. },
    ) = (sub.kind(), sup.kind())
    else {
        return false;
    };

    // Equal inclusive number bounds admit at most one JSON numeric value.  Ask
    // the resolved schemas directly so multipleOf/enum details (and the
    // validator's numeric semantics) stay authoritative.
    if let Some(value) = singleton_integer_number_value(*sub_bounds) {
        let json_value = Value::Number(value.into());
        return !sub.accepts_value(&json_value) || sup.accepts_value(&json_value);
    }

    // A number enum is finite, so we can prove cross-type inclusion by checking
    // every actually-admitted enum member directly.  This is deliberately more
    // conservative than reasoning from an integral multipleOf: the validator
    // applies an epsilon tolerance for numeric multipleOf, so e.g. values very
    // close to 1 can satisfy `multipleOf: 1` without being integer instances.
    if let Some(enum_values) = sub_enum.as_deref() {
        return enum_values
            .iter()
            .filter(|value| sub.accepts_value(value))
            .all(|value| sup.accepts_value(value));
    }

    false
}

fn singleton_integer_number_value(bounds: NumberBounds) -> Option<i64> {
    let (NumberBound::Inclusive(lower), NumberBound::Inclusive(upper)) =
        (bounds.lower(), bounds.upper())
    else {
        return None;
    };
    if lower != upper || lower.fract() != 0.0 {
        return None;
    }
    finite_bound_to_i64(lower)
}

fn finite_bound_to_i64(value: f64) -> Option<i64> {
    const MAX_EXACT_F64_INT: f64 = 9_007_199_254_740_992.0; // 2^53
    if !value.is_finite()
        || value.fract() != 0.0
        || !(-MAX_EXACT_F64_INT..=MAX_EXACT_F64_INT).contains(&value)
    {
        return None;
    }
    Some(value as i64)
}

pub(super) fn check_enum_inclusion(sub_enum: Option<&[Value]>, sup_enum: Option<&[Value]>) -> bool {
    match (sub_enum, sup_enum) {
        (_, None) => true,
        (Some(sub_enum), Some(sup_enum)) => sub_enum.iter().all(|value| {
            sup_enum
                .iter()
                .any(|expected| json_values_equal(expected, value))
        }),
        (None, Some(_)) => false,
    }
}

fn check_multiple_of_inclusion(
    sub_multiple_of: Option<&NumberMultipleOf>,
    sup_multiple_of: Option<&NumberMultipleOf>,
) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };

    sub_multiple_of
        .integer_divisor_is_multiple_of(*sup_multiple_of)
        .unwrap_or(false)
}

fn check_integer_multiple_of_inclusion(
    sub_multiple_of: Option<&IntegerMultipleOf>,
    sup_multiple_of: Option<&IntegerMultipleOf>,
) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };

    sub_multiple_of
        .integer_divisor_is_multiple_of(*sup_multiple_of)
        .unwrap_or(false)
}

fn check_integer_multiple_of_inclusion_by_number(
    sub_multiple_of: Option<&IntegerMultipleOf>,
    sup_multiple_of: Option<&NumberMultipleOf>,
) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };

    sub_multiple_of
        .integer_divisor_is_multiple_of_number(*sup_multiple_of)
        .unwrap_or(false)
}

fn required_constraint_is_preserved<T: PartialEq + ?Sized>(
    sub_constraint: Option<&T>,
    sup_constraint: Option<&T>,
) -> bool {
    sup_constraint.is_none() || sub_constraint == sup_constraint
}
