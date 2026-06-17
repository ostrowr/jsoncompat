//! Numeric and length interval helpers for conservative subset/disjointness proofs.
//!
//! These helpers only return positive facts when bounds are soundly known.

use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct NumericIntervalBound {
    pub(super) value: f64,
    pub(super) inclusive: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct NumericInterval {
    pub(super) lower: Option<NumericIntervalBound>,
    pub(super) upper: Option<NumericIntervalBound>,
    pub(super) empty: bool,
}

impl NumericInterval {
    const fn unbounded() -> Self {
        Self {
            lower: None,
            upper: None,
            empty: false,
        }
    }

    const fn empty() -> Self {
        Self {
            lower: Some(NumericIntervalBound {
                value: 0.0,
                inclusive: false,
            }),
            upper: Some(NumericIntervalBound {
                value: 0.0,
                inclusive: false,
            }),
            empty: true,
        }
    }

    fn intersect(mut self, other: Self) -> Self {
        if self.empty || other.empty {
            self.empty = true;
            return self;
        }
        self.lower = tighter_lower(self.lower, other.lower);
        self.upper = tighter_upper(self.upper, other.upper);
        if interval_bounds_are_empty(self.lower, self.upper) {
            self.empty = true;
        }
        self
    }

    fn hull(self, other: Self) -> Option<Self> {
        if self.empty {
            return Some(other);
        }
        if other.empty {
            return Some(self);
        }
        Some(Self {
            lower: looser_lower(self.lower, other.lower)?,
            upper: looser_upper(self.upper, other.upper)?,
            empty: false,
        })
    }
}

pub(super) fn tighter_lower(
    left: Option<NumericIntervalBound>,
    right: Option<NumericIntervalBound>,
) -> Option<NumericIntervalBound> {
    match (left, right) {
        (None, bound) | (bound, None) => bound,
        (Some(left), Some(right)) => match left.value.partial_cmp(&right.value) {
            Some(std::cmp::Ordering::Less) => Some(right),
            Some(std::cmp::Ordering::Greater) => Some(left),
            Some(std::cmp::Ordering::Equal) => Some(NumericIntervalBound {
                value: left.value,
                inclusive: left.inclusive && right.inclusive,
            }),
            None => None,
        },
    }
}

pub(super) fn tighter_upper(
    left: Option<NumericIntervalBound>,
    right: Option<NumericIntervalBound>,
) -> Option<NumericIntervalBound> {
    match (left, right) {
        (None, bound) | (bound, None) => bound,
        (Some(left), Some(right)) => match left.value.partial_cmp(&right.value) {
            Some(std::cmp::Ordering::Less) => Some(left),
            Some(std::cmp::Ordering::Greater) => Some(right),
            Some(std::cmp::Ordering::Equal) => Some(NumericIntervalBound {
                value: left.value,
                inclusive: left.inclusive && right.inclusive,
            }),
            None => None,
        },
    }
}

pub(super) fn looser_lower(
    left: Option<NumericIntervalBound>,
    right: Option<NumericIntervalBound>,
) -> Option<Option<NumericIntervalBound>> {
    match (left, right) {
        (None, _) | (_, None) => Some(None),
        (Some(left), Some(right)) => match left.value.partial_cmp(&right.value) {
            Some(std::cmp::Ordering::Less) => Some(Some(left)),
            Some(std::cmp::Ordering::Greater) => Some(Some(right)),
            Some(std::cmp::Ordering::Equal) => Some(Some(NumericIntervalBound {
                value: left.value,
                inclusive: left.inclusive || right.inclusive,
            })),
            None => None,
        },
    }
}

pub(super) fn looser_upper(
    left: Option<NumericIntervalBound>,
    right: Option<NumericIntervalBound>,
) -> Option<Option<NumericIntervalBound>> {
    match (left, right) {
        (None, _) | (_, None) => Some(None),
        (Some(left), Some(right)) => match left.value.partial_cmp(&right.value) {
            Some(std::cmp::Ordering::Less) => Some(Some(right)),
            Some(std::cmp::Ordering::Greater) => Some(Some(left)),
            Some(std::cmp::Ordering::Equal) => Some(Some(NumericIntervalBound {
                value: left.value,
                inclusive: left.inclusive || right.inclusive,
            })),
            None => None,
        },
    }
}

pub(super) fn interval_bounds_are_empty(
    lower: Option<NumericIntervalBound>,
    upper: Option<NumericIntervalBound>,
) -> bool {
    let (Some(lower), Some(upper)) = (lower, upper) else {
        return false;
    };
    match lower.value.partial_cmp(&upper.value) {
        Some(std::cmp::Ordering::Greater) => true,
        Some(std::cmp::Ordering::Equal) => !(lower.inclusive && upper.inclusive),
        _ => false,
    }
}

pub(super) fn numeric_intervals_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    let Some(left_interval) = numeric_interval_bound(left) else {
        return false;
    };
    let Some(right_interval) = numeric_interval_bound(right) else {
        return false;
    };
    if left_interval.empty || right_interval.empty {
        return true;
    }
    interval_strictly_before(left_interval, right_interval)
        || interval_strictly_before(right_interval, left_interval)
}

/// Prove disjointness for integer(-constrained) schemas whose `multipleOf`
/// constraints share no common multiple inside their overlapping finite
/// integer interval. Integer `multipleOf` constraints are all anchored at zero,
/// so their intersection is exactly the multiples of lcm(d1, d2). This is a
/// deliberately narrow syntactic check; returning false is conservative.
pub(super) fn integer_lattices_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    #[derive(Clone, Copy)]
    struct Summary {
        lower: Option<i64>,
        upper: Option<i64>,
        divisor: i128,
    }

    fn gcd(mut a: i128, mut b: i128) -> i128 {
        while b != 0 {
            let r = a.rem_euclid(b);
            a = b;
            b = r;
        }
        a.abs()
    }

    fn lcm(a: i128, b: i128) -> Option<i128> {
        if a <= 0 || b <= 0 {
            return None;
        }
        let g = gcd(a, b);
        a.checked_div(g)?.checked_mul(b)
    }

    fn intersect(left: Summary, right: Summary) -> Option<Summary> {
        let lower = match (left.lower, right.lower) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        let upper = match (left.upper, right.upper) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        Some(Summary {
            lower,
            upper,
            divisor: lcm(left.divisor, right.divisor)?,
        })
    }

    fn summarize(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<Summary> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::Integer {
                bounds,
                multiple_of,
                ..
            } => {
                let divisor = multiple_of
                    .as_ref()
                    .and_then(|multiple| multiple.integer_divisor())
                    .unwrap_or(1);
                (divisor > 0).then_some(Summary {
                    lower: bounds.lower(),
                    upper: bounds.upper(),
                    divisor,
                })
            }
            SchemaNodeKind::AllOf(children) => {
                let mut summary: Option<Summary> = None;
                for child in children {
                    if let Some(child_summary) = summarize(child, active) {
                        summary = Some(match summary {
                            Some(current) => match intersect(current, child_summary) {
                                Some(joined) => joined,
                                None => {
                                    active.remove(&schema.id());
                                    return None;
                                }
                            },
                            None => child_summary,
                        });
                    }
                }
                summary
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    fn ceil_div(n: i128, d: i128) -> i128 {
        debug_assert!(d > 0);
        let q = n.div_euclid(d);
        let r = n.rem_euclid(d);
        if r == 0 { q } else { q + 1 }
    }

    let Some(left) = summarize(left, &mut HashSet::new()) else {
        return false;
    };
    let Some(right) = summarize(right, &mut HashSet::new()) else {
        return false;
    };
    let Some(combined) = intersect(left, right) else {
        return false;
    };
    let lower = combined.lower.map(i128::from);
    let upper = combined.upper.map(i128::from);
    let (Some(lower), Some(upper)) = (lower, upper) else {
        // Any half-infinite interval contains some multiple of a positive lcm.
        return false;
    };
    if lower > upper {
        return true;
    }
    let first_multiple = ceil_div(lower, combined.divisor).saturating_mul(combined.divisor);
    first_multiple > upper
}

/// Cheap string-length disjointness for partition proofs. Length bounds are
/// monotone under `allOf`, so intersecting syntactic bounds gives an
/// over-approximation of each schema's string language. If those intervals do
/// not overlap, the schemas cannot share a string value. Callers only use this
/// when string is the only overlapping JSON type, since min/maxLength do not
/// reject non-strings by themselves.
pub(super) fn string_length_intervals_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    let Some(left_interval) = string_length_interval_bound(left) else {
        return false;
    };
    let Some(right_interval) = string_length_interval_bound(right) else {
        return false;
    };
    left_interval.empty
        || right_interval.empty
        || length_interval_strictly_before(left_interval, right_interval)
        || length_interval_strictly_before(right_interval, left_interval)
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LengthInterval {
    pub(super) lower: u64,
    pub(super) upper: Option<u64>,
    pub(super) empty: bool,
}

impl LengthInterval {
    const fn unbounded() -> Self {
        Self {
            lower: 0,
            upper: None,
            empty: false,
        }
    }

    const fn empty() -> Self {
        Self {
            lower: 1,
            upper: Some(0),
            empty: true,
        }
    }

    fn intersect(mut self, other: Self) -> Self {
        if self.empty || other.empty {
            self.empty = true;
            return self;
        }
        self.lower = self.lower.max(other.lower);
        self.upper = match (self.upper, other.upper) {
            (None, bound) | (bound, None) => bound,
            (Some(left), Some(right)) => Some(left.min(right)),
        };
        if self.upper.is_some_and(|upper| self.lower > upper) {
            self.empty = true;
        }
        self
    }

    /// Return an interval containing both inputs. This is used for union
    /// applicators: a hull is deliberately imprecise, but remains a sound
    /// over-approximation for disjointness proofs.
    fn hull(self, other: Self) -> Self {
        if self.empty {
            return other;
        }
        if other.empty {
            return self;
        }
        Self {
            lower: self.lower.min(other.lower),
            upper: match (self.upper, other.upper) {
                (Some(left), Some(right)) => Some(left.max(right)),
                _ => None,
            },
            empty: false,
        }
    }
}

pub(super) fn string_length_interval_bound(schema: &SchemaNode) -> Option<LengthInterval> {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<LengthInterval> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(LengthInterval {
                lower: 1,
                upper: Some(0),
                empty: true,
            }),
            SchemaNodeKind::String { length, .. } => Some(LengthInterval {
                lower: length.min(),
                upper: length.max(),
                empty: length.max().is_some_and(|upper| length.min() > upper),
            }),
            SchemaNodeKind::Const(value) => {
                if let Some(string) = value.as_str() {
                    let len = u64::try_from(string.chars().count()).ok()?;
                    Some(LengthInterval {
                        lower: len,
                        upper: Some(len),
                        empty: false,
                    })
                } else {
                    Some(LengthInterval::empty())
                }
            }
            SchemaNodeKind::Enum(values) => {
                let mut lower: Option<u64> = None;
                let mut upper: Option<u64> = None;
                for value in values {
                    if let Some(string) = value.as_str() {
                        let len = u64::try_from(string.chars().count()).ok()?;
                        lower = Some(lower.map_or(len, |current| current.min(len)));
                        upper = Some(upper.map_or(len, |current| current.max(len)));
                    }
                }
                match (lower, upper) {
                    (Some(lower), Some(upper)) => Some(LengthInterval {
                        lower,
                        upper: Some(upper),
                        empty: false,
                    }),
                    _ => Some(LengthInterval::empty()),
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    match (inner(then_schema, active), inner(else_schema, active)) {
                        (Some(then_interval), Some(else_interval)) => {
                            Some(then_interval.hull(else_interval))
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_STRING != 0 =>
                {
                    inner(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_STRING == 0 =>
                {
                    inner(else_schema, active)
                }
                _ => None,
            },
            SchemaNodeKind::AllOf(children) => {
                let mut interval = LengthInterval::unbounded();
                let mut saw_bound = false;
                for child in children {
                    if let Some(child_interval) = inner(child, active) {
                        interval = interval.intersect(child_interval);
                        saw_bound = true;
                    }
                }
                saw_bound.then_some(interval)
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut hull: Option<LengthInterval> = None;
                let mut unknown_string_branch = false;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_STRING == 0 {
                        continue;
                    }
                    match inner(child, active) {
                        Some(child_interval) => {
                            hull =
                                Some(hull.map_or(child_interval, |acc| acc.hull(child_interval)));
                        }
                        None => {
                            unknown_string_branch = true;
                            break;
                        }
                    }
                }
                if unknown_string_branch {
                    None
                } else {
                    Some(hull.unwrap_or_else(LengthInterval::empty))
                }
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

pub(super) fn length_interval_strictly_before(left: LengthInterval, right: LengthInterval) -> bool {
    if left.empty || right.empty {
        return true;
    }
    left.upper.is_some_and(|upper| upper < right.lower)
}

/// Same interval proof as string lengths, for minItems/maxItems.  This is
/// only consulted when arrays are the sole overlapping JSON type.
pub(super) fn array_length_intervals_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    let Some(left_interval) = array_length_interval_bound(left) else {
        return false;
    };
    let Some(right_interval) = array_length_interval_bound(right) else {
        return false;
    };
    left_interval.empty
        || right_interval.empty
        || length_interval_strictly_before(left_interval, right_interval)
        || length_interval_strictly_before(right_interval, left_interval)
}

pub(super) fn array_length_interval_bound(schema: &SchemaNode) -> Option<LengthInterval> {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<LengthInterval> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(LengthInterval {
                lower: 1,
                upper: Some(0),
                empty: true,
            }),
            SchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                ..
            } => {
                let effective_count = array::effective_item_count_for_unique_finite_domain(
                    prefix_items,
                    items,
                    *item_count,
                    *unique_items,
                )
                .unwrap_or(*item_count);
                let mut lower = effective_count.min();
                let mut upper = effective_count.max();
                let mut empty = upper.is_some_and(|upper| lower > upper);

                if let Some(contains) = contains {
                    let count = contains.count();
                    // At least N matching items implies at least N items total.
                    lower = lower.max(count.min());
                    // If every item necessarily matches (the common
                    // `contains: true` spelling), match-count bounds are also
                    // length bounds. Conversely, an impossible contains schema
                    // with a positive minimum makes the array language empty.
                    if schema_is_trivially_universal(&contains.schema) {
                        upper = match (upper, count.max()) {
                            (Some(a), Some(b)) => Some(a.min(b)),
                            (None, bound) | (bound, None) => bound,
                        };
                    } else if count.min() > 0
                        && schema_is_locally_empty_for_finite_enumeration(&contains.schema)
                    {
                        empty = true;
                    }
                    if upper.is_some_and(|upper| lower > upper) {
                        empty = true;
                    }
                }

                Some(LengthInterval {
                    lower,
                    upper,
                    empty,
                })
            }
            SchemaNodeKind::Const(value) => {
                if let Some(array) = value.as_array() {
                    let len = u64::try_from(array.len()).ok()?;
                    Some(LengthInterval {
                        lower: len,
                        upper: Some(len),
                        empty: false,
                    })
                } else {
                    Some(LengthInterval::empty())
                }
            }
            SchemaNodeKind::Enum(values) => {
                let mut lower: Option<u64> = None;
                let mut upper: Option<u64> = None;
                for value in values {
                    if let Some(array) = value.as_array() {
                        let len = u64::try_from(array.len()).ok()?;
                        lower = Some(lower.map_or(len, |current| current.min(len)));
                        upper = Some(upper.map_or(len, |current| current.max(len)));
                    }
                }
                match (lower, upper) {
                    (Some(lower), Some(upper)) => Some(LengthInterval {
                        lower,
                        upper: Some(upper),
                        empty: false,
                    }),
                    _ => Some(LengthInterval::empty()),
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    match (inner(then_schema, active), inner(else_schema, active)) {
                        (Some(then_interval), Some(else_interval)) => {
                            Some(then_interval.hull(else_interval))
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    inner(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    inner(else_schema, active)
                }
                _ => None,
            },
            SchemaNodeKind::AllOf(children) => {
                let mut interval = LengthInterval::unbounded();
                let mut saw_bound = false;
                for child in children {
                    if let Some(child_interval) = inner(child, active) {
                        interval = interval.intersect(child_interval);
                        saw_bound = true;
                    }
                }
                saw_bound.then_some(interval)
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut hull: Option<LengthInterval> = None;
                let mut unknown_array_branch = false;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_ARRAY == 0 {
                        continue;
                    }
                    match inner(child, active) {
                        Some(child_interval) => {
                            hull =
                                Some(hull.map_or(child_interval, |acc| acc.hull(child_interval)));
                        }
                        None => {
                            unknown_array_branch = true;
                            break;
                        }
                    }
                }
                if unknown_array_branch {
                    None
                } else {
                    Some(hull.unwrap_or_else(LengthInterval::empty))
                }
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

/// Same interval proof for minProperties/maxProperties, used only when objects
/// are the sole overlapping JSON type.
pub(super) fn object_property_count_intervals_are_disjoint(
    left: &SchemaNode,
    right: &SchemaNode,
) -> bool {
    let Some(left_interval) = object_property_count_interval_bound(left) else {
        return false;
    };
    let Some(right_interval) = object_property_count_interval_bound(right) else {
        return false;
    };
    left_interval.empty
        || right_interval.empty
        || length_interval_strictly_before(left_interval, right_interval)
        || length_interval_strictly_before(right_interval, left_interval)
}

pub(super) fn object_property_count_interval_bound(schema: &SchemaNode) -> Option<LengthInterval> {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<LengthInterval> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(LengthInterval {
                lower: 1,
                upper: Some(0),
                empty: true,
            }),
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                property_names,
                property_count,
                ..
            } => {
                let syntactic_lower = u64::try_from(property_count.min()).ok()?;
                let implied_lower =
                    u64::try_from(guaranteed_property_name_closure(schema).len()).ok()?;
                let lower = syntactic_lower.max(implied_lower);
                let mut upper = property_count.max().map(u64::try_from).transpose().ok()?;

                // Closed objects with no pattern properties can only use the
                // explicitly declared names. This is a hard capacity even when
                // individual property schemas are broad or recursive.
                if pattern_properties.is_empty()
                    && matches!(additional.kind(), SchemaNodeKind::BoolSchema(false))
                {
                    let usable_declared = properties
                        .keys()
                        .filter(|name| !schema_forbids_property_name_for_objects(schema, name))
                        .count();
                    let declared = u64::try_from(usable_declared).ok()?;
                    upper = Some(upper.map_or(declared, |current| current.min(declared)));
                }

                // A finite propertyNames language caps the number of distinct
                // keys regardless of additionalProperties/patternProperties.
                // `finite_schema_value_superset` is an upper bound, so counting
                // its string members remains a sound (possibly loose) capacity.
                if let Some(names) = finite_property_name_strings_superset(property_names) {
                    // Some members of a finite propertyNames language may be
                    // syntactically impossible anyway (for example a declared
                    // property with schema `false`, or a name rejected by a
                    // matching false patternProperty). Dropping names that
                    // this very object schema forbids gives a tighter capacity
                    // while remaining an upper bound: every usable key must be
                    // in the finite name language and must not be individually
                    // forbidden by the object constraints.
                    let usable = names
                        .iter()
                        .filter(|name| !schema_forbids_property_name_for_objects(schema, name))
                        .count();
                    let finite = u64::try_from(usable).ok()?;
                    upper = Some(upper.map_or(finite, |current| current.min(finite)));
                }

                Some(LengthInterval {
                    lower,
                    upper,
                    empty: upper.is_some_and(|upper| lower > upper),
                })
            }
            SchemaNodeKind::Const(value) => {
                if let Some(object) = value.as_object() {
                    let len = u64::try_from(object.len()).ok()?;
                    Some(LengthInterval {
                        lower: len,
                        upper: Some(len),
                        empty: false,
                    })
                } else {
                    Some(LengthInterval::empty())
                }
            }
            SchemaNodeKind::Enum(values) => {
                let mut lower: Option<u64> = None;
                let mut upper: Option<u64> = None;
                for value in values {
                    if let Some(object) = value.as_object() {
                        let len = u64::try_from(object.len()).ok()?;
                        lower = Some(lower.map_or(len, |current| current.min(len)));
                        upper = Some(upper.map_or(len, |current| current.max(len)));
                    }
                }
                match (lower, upper) {
                    (Some(lower), Some(upper)) => Some(LengthInterval {
                        lower,
                        upper: Some(upper),
                        empty: false,
                    }),
                    _ => Some(LengthInterval::empty()),
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    match (inner(then_schema, active), inner(else_schema, active)) {
                        (Some(then_interval), Some(else_interval)) => {
                            Some(then_interval.hull(else_interval))
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    inner(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    inner(else_schema, active)
                }
                _ => None,
            },
            SchemaNodeKind::AllOf(children) => {
                let mut interval = LengthInterval::unbounded();
                let mut saw_bound = false;
                for child in children {
                    if let Some(child_interval) = inner(child, active) {
                        interval = interval.intersect(child_interval);
                        saw_bound = true;
                    }
                }
                if saw_bound {
                    if let Ok(implied_lower) =
                        u64::try_from(guaranteed_property_name_closure(schema).len())
                    {
                        interval.lower = interval.lower.max(implied_lower);
                        if interval.upper.is_some_and(|upper| interval.lower > upper) {
                            interval.empty = true;
                        }
                    }
                    Some(interval)
                } else {
                    None
                }
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut hull: Option<LengthInterval> = None;
                let mut unknown_object_branch = false;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_OBJECT == 0 {
                        continue;
                    }
                    match inner(child, active) {
                        Some(child_interval) => {
                            hull =
                                Some(hull.map_or(child_interval, |acc| acc.hull(child_interval)));
                        }
                        None => {
                            unknown_object_branch = true;
                            break;
                        }
                    }
                }
                if unknown_object_branch {
                    None
                } else {
                    Some(hull.unwrap_or_else(LengthInterval::empty))
                }
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

/// Return the exact small set of integer JSON values accepted by a `type:
/// number` schema when that finiteness is syntactically obvious.  We accept
/// explicit integer enums and inclusive singleton integer bounds.  In
/// particular, do *not* infer integrality from numeric `multipleOf`: validators
/// commonly use floating tolerances around multiples, so that would be unsafe.
/// Callers use this to bridge normalized numeric singletons into integer/const
/// targets without general arithmetic reasoning.
pub(super) fn finite_integer_number_values(schema: &SchemaNode) -> Option<Vec<Value>> {
    let SchemaNodeKind::Number {
        multiple_of: _,
        enumeration,
        ..
    } = schema.kind()
    else {
        return None;
    };
    if let Some(values) = enumeration {
        if values.iter().all(|value| value.as_i64().is_some()) {
            return Some(values.clone());
        }
        return None;
    }

    let interval = numeric_interval_bound(schema)?;
    if interval.empty {
        return Some(Vec::new());
    }
    let (Some(lower), Some(upper)) = (interval.lower, interval.upper) else {
        return None;
    };
    if !lower.value.is_finite() || !upper.value.is_finite() {
        return None;
    }
    const MAX_EXACT: f64 = 9_007_199_254_740_991.0;
    if lower.value < -MAX_EXACT || upper.value > MAX_EXACT {
        return None;
    }

    // Do not infer integrality from `multipleOf` on a number schema: both the
    // JSON Schema validator and our evaluator allow tiny floating tolerances
    // around numeric multiples.  Only an inclusive singleton bound (or an
    // explicit enum handled above) is an exact finite integer language.
    let integrality_forced = lower.inclusive
        && upper.inclusive
        && lower.value == upper.value
        && lower.value.fract() == 0.0;
    if !integrality_forced {
        return None;
    }

    let mut start = lower.value.ceil();
    if !lower.inclusive && start == lower.value {
        start += 1.0;
    }
    let mut end = upper.value.floor();
    if !upper.inclusive && end == upper.value {
        end -= 1.0;
    }
    if start > end {
        return Some(Vec::new());
    }
    if start < i64::MIN as f64 || end > i64::MAX as f64 {
        return None;
    }
    let start_i = start as i64;
    let end_i = end as i64;
    if end_i.checked_sub(start_i).is_none_or(|span| span > 2048) {
        return None;
    }

    let mut values = Vec::new();
    for n in start_i..=end_i {
        let value = Value::Number(n.into());
        if schema.accepts_value(&value) {
            values.push(value);
        }
    }
    Some(values)
}

/// Return a small finite integer upper bound for split `allOf` integer ranges.
/// This is deliberately an over-approximation: endpoints are rounded outward,
/// then callers filter candidates with the exact/rejection evaluator. The
/// direct-integer-conjunct guard is what makes it safe to enumerate integers
/// rather than arbitrary numbers.
pub(super) fn finite_split_allof_integer_values(schema: &SchemaNode) -> Option<Vec<Value>> {
    if !matches!(schema.kind(), SchemaNodeKind::AllOf(_))
        || !allof_has_direct_integer_conjunct(schema)
        || possible_json_type_mask(schema) & !JSON_TYPE_NUMBER != 0
    {
        return None;
    }
    let interval = numeric_interval_bound(schema)?;
    if interval.empty {
        return Some(Vec::new());
    }
    let (Some(lower), Some(upper)) = (interval.lower, interval.upper) else {
        return None;
    };
    if !lower.value.is_finite() || !upper.value.is_finite() {
        return None;
    }
    const MAX_EXACT: f64 = 9_007_199_254_740_991.0;
    if lower.value < -MAX_EXACT || upper.value > MAX_EXACT {
        return None;
    }
    // Round outward; this may include one extra integer at an exclusive bound,
    // which is fine for an upper bound.
    let start_f = lower.value.floor();
    let end_f = upper.value.ceil();
    if start_f < i64::MIN as f64 || end_f > i64::MAX as f64 || start_f > end_f {
        return None;
    }
    let start = start_f as i64;
    let end = end_f as i64;
    if end.checked_sub(start).is_none_or(|span| span > 256) {
        return None;
    }
    Some((start..=end).map(|n| Value::Number(n.into())).collect())
}

/// Prove a split `allOf` numeric intersection is contained by a plain number
/// range. This is intentionally narrower than general numeric implication:
/// callers only use it for a right-hand `Number` without `multipleOf` or
/// enumeration. The type-mask check is essential because JSON Schema numeric
/// bounds alone do not reject non-number instances.
pub(super) fn split_allof_numeric_range_subset_of_number(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> bool {
    let sub_mask = possible_json_type_mask(sub);
    if sub_mask == 0 {
        return true;
    }
    if sub_mask & !JSON_TYPE_NUMBER != 0 {
        return false;
    }
    let Some(sub_interval) = numeric_interval_bound(sub) else {
        return false;
    };
    if sub_interval.empty {
        return true;
    }
    let Some(sup_interval) = numeric_interval_bound(sup) else {
        return false;
    };
    numeric_interval_contains(sup_interval, sub_interval)
}

pub(super) fn split_allof_integer_range_subset_of_integer(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> bool {
    let sub_mask = possible_json_type_mask(sub);
    if sub_mask == 0 {
        return true;
    }
    if sub_mask & !JSON_TYPE_NUMBER != 0 {
        return false;
    }
    if !allof_has_direct_integer_conjunct(sub) {
        return false;
    }
    let Some(sub_interval) = numeric_interval_bound(sub) else {
        return false;
    };
    if sub_interval.empty {
        return true;
    }
    let Some(sup_interval) = numeric_interval_bound(sup) else {
        return false;
    };
    numeric_interval_contains(sup_interval, sub_interval)
}

pub(super) fn allof_has_direct_integer_conjunct(schema: &SchemaNode) -> bool {
    match schema.kind() {
        SchemaNodeKind::AllOf(children) => children
            .iter()
            .any(|child| matches!(child.kind(), SchemaNodeKind::Integer { .. })),
        _ => false,
    }
}

pub(super) fn split_allof_string_length_subset_of_string(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> bool {
    split_allof_length_subset_of_type(sub, sup, JSON_TYPE_STRING, string_length_interval_bound)
}

pub(super) fn split_allof_array_length_subset_of_array(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    split_allof_length_subset_of_type(sub, sup, JSON_TYPE_ARRAY, array_length_interval_bound)
}

pub(super) fn split_allof_object_count_subset_of_object(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> bool {
    split_allof_length_subset_of_type(
        sub,
        sup,
        JSON_TYPE_OBJECT,
        object_property_count_interval_bound,
    )
}

pub(super) fn split_allof_length_subset_of_type(
    sub: &SchemaNode,
    sup: &SchemaNode,
    type_bit: u8,
    interval_bound: fn(&SchemaNode) -> Option<LengthInterval>,
) -> bool {
    let sub_mask = possible_json_type_mask(sub);
    if sub_mask == 0 {
        return true;
    }
    if sub_mask & !type_bit != 0 {
        return false;
    }
    let Some(sub_interval) = interval_bound(sub) else {
        return false;
    };
    if sub_interval.empty {
        return true;
    }
    let Some(sup_interval) = interval_bound(sup) else {
        return false;
    };
    length_interval_contains(sup_interval, sub_interval)
}

pub(super) fn length_interval_contains(outer: LengthInterval, inner: LengthInterval) -> bool {
    if inner.empty {
        return true;
    }
    if outer.empty {
        return false;
    }
    if outer.lower > inner.lower {
        return false;
    }
    match (outer.upper, inner.upper) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(outer_upper), Some(inner_upper)) => inner_upper <= outer_upper,
    }
}

pub(super) fn array_schema_is_plain_count(schema: &SchemaNode) -> bool {
    match schema.kind() {
        SchemaNodeKind::Array {
            prefix_items,
            items,
            contains,
            unique_items,
            enumeration,
            ..
        } => {
            prefix_items.is_empty()
                && schema_is_trivially_universal(items)
                && contains.is_none()
                && !*unique_items
                && enumeration.is_none()
        }
        _ => false,
    }
}

pub(super) fn object_schema_is_plain_count(schema: &SchemaNode) -> bool {
    match schema.kind() {
        SchemaNodeKind::Object {
            properties,
            pattern_properties,
            required,
            additional,
            property_names,
            dependent_required,
            enumeration,
            ..
        } => {
            properties.is_empty()
                && pattern_properties.is_empty()
                && required.is_empty()
                && schema_is_trivially_universal(additional)
                && schema_is_trivially_universal(property_names)
                && dependent_required.is_empty()
                && enumeration.is_none()
        }
        _ => false,
    }
}

pub(super) fn integer_schema_is_plain_range(schema: &SchemaNode) -> bool {
    match schema.kind() {
        SchemaNodeKind::Integer {
            multiple_of,
            enumeration,
            ..
        } => {
            enumeration.is_none()
                && multiple_of
                    .as_ref()
                    .is_none_or(|multiple| multiple.integer_divisor() == Some(1))
        }
        _ => false,
    }
}

pub(super) fn numeric_interval_contains(outer: NumericInterval, inner: NumericInterval) -> bool {
    if inner.empty {
        return true;
    }
    if outer.empty {
        return false;
    }
    lower_bound_contains(outer.lower, inner.lower) && upper_bound_contains(outer.upper, inner.upper)
}

pub(super) fn lower_bound_contains(
    outer: Option<NumericIntervalBound>,
    inner: Option<NumericIntervalBound>,
) -> bool {
    match (outer, inner) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(outer), Some(inner)) => match inner.value.partial_cmp(&outer.value) {
            Some(std::cmp::Ordering::Greater) => true,
            Some(std::cmp::Ordering::Equal) => outer.inclusive || !inner.inclusive,
            _ => false,
        },
    }
}

pub(super) fn upper_bound_contains(
    outer: Option<NumericIntervalBound>,
    inner: Option<NumericIntervalBound>,
) -> bool {
    match (outer, inner) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(outer), Some(inner)) => match inner.value.partial_cmp(&outer.value) {
            Some(std::cmp::Ordering::Less) => true,
            Some(std::cmp::Ordering::Equal) => outer.inclusive || !inner.inclusive,
            _ => false,
        },
    }
}

pub(super) fn interval_strictly_before(left: NumericInterval, right: NumericInterval) -> bool {
    let (Some(left_upper), Some(right_lower)) = (left.upper, right.lower) else {
        return false;
    };
    match left_upper.value.partial_cmp(&right_lower.value) {
        Some(std::cmp::Ordering::Less) => true,
        Some(std::cmp::Ordering::Equal) => !(left_upper.inclusive && right_lower.inclusive),
        _ => false,
    }
}

pub(super) fn numeric_interval_bound(schema: &SchemaNode) -> Option<NumericInterval> {
    fn from_number_bound(bound: NumberBound) -> Option<NumericIntervalBound> {
        match bound {
            NumberBound::Unbounded => None,
            NumberBound::Inclusive(value) => Some(NumericIntervalBound {
                value,
                inclusive: true,
            }),
            NumberBound::Exclusive(value) => Some(NumericIntervalBound {
                value,
                inclusive: false,
            }),
        }
    }

    fn integer_endpoint(value: i64) -> Option<f64> {
        const MAX_EXACT: i64 = 9_007_199_254_740_991;
        value
            .checked_abs()
            .is_some_and(|abs| abs <= MAX_EXACT)
            .then_some(value as f64)
    }

    // Convert JSON numeric literals only when doing so cannot silently widen or
    // narrow a large integer. The interval proof is allowed to be imprecise,
    // but not wrong: a rounded 2^63 literal could otherwise look equal to a
    // neighboring value and create a bogus disjointness fact.
    fn json_number_endpoint(value: &Value) -> Option<f64> {
        let number = value.as_number()?;
        const MAX_EXACT_U64: u64 = 9_007_199_254_740_991;
        if let Some(integer) = number.as_i64() {
            return integer_endpoint(integer);
        }
        if let Some(integer) = number.as_u64() {
            return (integer <= MAX_EXACT_U64).then_some(integer as f64);
        }
        let value = number.as_f64()?;
        value.is_finite().then_some(value)
    }

    // Return a hull for the numeric members of a literal set. Non-numeric
    // members are ignored (the caller separately checks type-mask overlap),
    // while an unrepresentable numeric member makes the hull unknown.
    fn literal_numeric_hull(values: &[Value]) -> Option<NumericInterval> {
        let mut hull: Option<NumericInterval> = None;
        let mut saw_numeric = false;
        for value in values {
            if !value.is_number() {
                continue;
            }
            saw_numeric = true;
            let endpoint = json_number_endpoint(value)?;
            let singleton = NumericInterval {
                lower: Some(NumericIntervalBound {
                    value: endpoint,
                    inclusive: true,
                }),
                upper: Some(NumericIntervalBound {
                    value: endpoint,
                    inclusive: true,
                }),
                empty: false,
            };
            hull = Some(match hull {
                Some(current) => current.hull(singleton)?,
                None => singleton,
            });
        }
        Some(if saw_numeric {
            hull.unwrap_or_else(NumericInterval::empty)
        } else {
            NumericInterval::empty()
        })
    }

    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<NumericInterval> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(NumericInterval::empty()),
            SchemaNodeKind::Const(value) => {
                if value.is_number() {
                    json_number_endpoint(value).map(|endpoint| NumericInterval {
                        lower: Some(NumericIntervalBound {
                            value: endpoint,
                            inclusive: true,
                        }),
                        upper: Some(NumericIntervalBound {
                            value: endpoint,
                            inclusive: true,
                        }),
                        empty: false,
                    })
                } else {
                    Some(NumericInterval::empty())
                }
            }
            SchemaNodeKind::Enum(values) => literal_numeric_hull(values),
            SchemaNodeKind::Number {
                bounds,
                enumeration,
                ..
            } => {
                let mut interval = NumericInterval {
                    lower: from_number_bound(bounds.lower()),
                    upper: from_number_bound(bounds.upper()),
                    empty: false,
                };
                if let Some(values) = enumeration
                    && let Some(enum_interval) = literal_numeric_hull(values)
                {
                    interval = interval.intersect(enum_interval);
                }
                Some(interval)
            }
            SchemaNodeKind::Integer {
                bounds,
                enumeration,
                ..
            } => {
                let lower = match bounds.lower() {
                    Some(value) => match integer_endpoint(value) {
                        Some(value) => Some(NumericIntervalBound {
                            value,
                            inclusive: true,
                        }),
                        None => {
                            active.remove(&schema.id());
                            return None;
                        }
                    },
                    None => None,
                };
                let upper = match bounds.upper() {
                    Some(value) => match integer_endpoint(value) {
                        Some(value) => Some(NumericIntervalBound {
                            value,
                            inclusive: true,
                        }),
                        None => {
                            active.remove(&schema.id());
                            return None;
                        }
                    },
                    None => None,
                };
                let mut interval = NumericInterval {
                    lower,
                    upper,
                    empty: false,
                };
                if let Some(values) = enumeration
                    && let Some(enum_interval) = literal_numeric_hull(values)
                {
                    interval = interval.intersect(enum_interval);
                }
                Some(interval)
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    match (inner(then_schema, active), inner(else_schema, active)) {
                        (Some(then_interval), Some(else_interval)) => {
                            then_interval.hull(else_interval)
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_NUMBER != 0 =>
                {
                    inner(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_NUMBER == 0 =>
                {
                    inner(else_schema, active)
                }
                _ => None,
            },
            SchemaNodeKind::AllOf(children) => {
                let mut interval = NumericInterval::unbounded();
                let mut saw_numeric_bound = false;
                for child in children {
                    if let Some(child_interval) = inner(child, active) {
                        interval = interval.intersect(child_interval);
                        saw_numeric_bound = true;
                    }
                }
                saw_numeric_bound.then_some(interval)
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut hull: Option<NumericInterval> = None;
                let mut unknown_numeric_branch = false;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_NUMBER == 0 {
                        continue;
                    }
                    match inner(child, active) {
                        Some(child_interval) => {
                            hull = match hull {
                                Some(acc) => match acc.hull(child_interval) {
                                    Some(joined) => Some(joined),
                                    None => {
                                        unknown_numeric_branch = true;
                                        break;
                                    }
                                },
                                None => Some(child_interval),
                            };
                        }
                        None => {
                            unknown_numeric_branch = true;
                            break;
                        }
                    }
                }
                if unknown_numeric_branch {
                    None
                } else {
                    Some(hull.unwrap_or_else(NumericInterval::empty))
                }
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}
