//! Scalar subset helpers for numbers, integers, strings, booleans, and nulls.

use crate::SchemaNode;
use json_schema_ast::{
    CountRange, IntegerBounds, IntegerMultipleOf, NumberBounds, SchemaNodeKind, json_values_equal,
};
use serde_json::Value;

pub(super) fn scalar_constraints_subsumed(
    sub_length: CountRange<u64>,
    sub_enum: Option<&[Value]>,
    sup_length: CountRange<u64>,
    sup_enum: Option<&[Value]>,
) -> bool {
    sup_length.contains_range(sub_length) && check_enum_inclusion(sub_enum, sup_enum)
}

pub(super) fn number_constraints_subsumed(
    sub_bounds: NumberBounds,
    sub_multiple_of: Option<f64>,
    sub_enum: Option<&[Value]>,
    sup_bounds: NumberBounds,
    sup_multiple_of: Option<f64>,
    sup_enum: Option<&[Value]>,
) -> bool {
    sup_bounds.contains_bounds(sub_bounds)
        && check_multiple_of_inclusion(sub_multiple_of, sup_multiple_of)
        && check_enum_inclusion(sub_enum, sup_enum)
}

pub(super) fn integer_constraints_subsumed(
    sub_bounds: IntegerBounds,
    sub_multiple_of: Option<IntegerMultipleOf>,
    sub_enum: Option<&[Value]>,
    sup_bounds: IntegerBounds,
    sup_multiple_of: Option<IntegerMultipleOf>,
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
        && check_integer_multiple_of_inclusion_by_number(*sub_multiple_of, *sup_multiple_of)
        && check_enum_inclusion(sub_enum.as_deref(), sup_enum.as_deref())
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

fn check_integer_multiple_of_inclusion(
    sub_multiple_of: Option<IntegerMultipleOf>,
    sup_multiple_of: Option<IntegerMultipleOf>,
) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };

    if let (Some(sub_divisor), Some(sup_divisor)) = (
        sub_multiple_of.integer_divisor(),
        sup_multiple_of.integer_divisor(),
    ) {
        return sub_divisor.rem_euclid(sup_divisor) == 0;
    }

    let ratio = sub_multiple_of.as_f64() / sup_multiple_of.as_f64();
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn check_integer_multiple_of_inclusion_by_number(
    sub_multiple_of: Option<IntegerMultipleOf>,
    sup_multiple_of: Option<f64>,
) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };
    if sup_multiple_of <= 0.0 {
        return false;
    }

    if let Some(sup_multiple_of) = exact_positive_integer(sup_multiple_of)
        && let Ok(sup_multiple_of) = i64::try_from(sup_multiple_of)
        && let Some(sub_multiple_of) = sub_multiple_of.integer_divisor()
    {
        return sub_multiple_of.rem_euclid(i128::from(sup_multiple_of)) == 0;
    }

    let ratio = sub_multiple_of.as_f64() / sup_multiple_of;
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn exact_positive_integer(value: f64) -> Option<u64> {
    if !value.is_finite() || value <= 0.0 || value.fract() != 0.0 || value > u64::MAX as f64 {
        return None;
    }

    let integer = value as u64;
    ((integer as f64) == value).then_some(integer)
}
