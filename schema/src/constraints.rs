//! Normalized constraint types used by the resolved schema IR.

use fancy_regex::Regex;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A canonical closed interval over signed 64-bit integers.
///
/// JSON Schema's exclusive integer bounds are normalized to inclusive endpoints
/// at parse time, so semantically identical intervals have one representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegerBounds {
    lower: Option<i64>,
    upper: Option<i64>,
}

impl IntegerBounds {
    /// Return an interval with no lower or upper endpoint.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    /// Return the inclusive lower endpoint, if present.
    #[must_use]
    pub const fn lower(self) -> Option<i64> {
        self.lower
    }

    /// Return the inclusive upper endpoint, if present.
    #[must_use]
    pub const fn upper(self) -> Option<i64> {
        self.upper
    }

    /// Build a validated closed integer interval.
    ///
    /// Returns `None` when both endpoints are present and `lower > upper`.
    #[must_use]
    pub fn new(lower: Option<i64>, upper: Option<i64>) -> Option<Self> {
        if let (Some(lower), Some(upper)) = (lower, upper)
            && lower > upper
        {
            return None;
        }

        Some(Self { lower, upper })
    }

    /// Return true when `value` is inside this interval.
    #[must_use]
    pub fn contains_i64(self, value: i64) -> bool {
        self.lower.is_none_or(|lower| value >= lower)
            && self.upper.is_none_or(|upper| value <= upper)
    }

    /// Return true when `value` is inside this interval.
    #[must_use]
    pub fn contains_i128(self, value: i128) -> bool {
        self.lower.is_none_or(|lower| value >= i128::from(lower))
            && self.upper.is_none_or(|upper| value <= i128::from(upper))
    }

    /// Return true when `sub` is wholly contained by this interval.
    #[must_use]
    pub fn contains_bounds(self, sub: Self) -> bool {
        self.lower <= sub.lower
            && match (self.upper, sub.upper) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(sup_upper), Some(sub_upper)) => sub_upper <= sup_upper,
            }
    }

    /// Project this integer interval into the corresponding number interval.
    #[must_use]
    pub fn as_number_bounds(self) -> NumberBounds {
        NumberBounds::new(
            self.lower.map_or(NumberBound::Unbounded, |value| {
                NumberBound::Inclusive(value as f64)
            }),
            self.upper.map_or(NumberBound::Unbounded, |value| {
                NumberBound::Inclusive(value as f64)
            }),
        )
        .expect("finite i64 endpoints must project to valid f64 number bounds")
    }

    pub(crate) fn from_json_schema_keywords(
        minimum: Option<i64>,
        exclusive_minimum: Option<i64>,
        maximum: Option<i64>,
        exclusive_maximum: Option<i64>,
    ) -> Option<Self> {
        let lower = if let Some(bound) = exclusive_minimum {
            Some(bound.checked_add(1)?)
        } else {
            minimum
        };
        let upper = if let Some(bound) = exclusive_maximum {
            Some(bound.checked_sub(1)?)
        } else {
            maximum
        };
        Self::new(lower, upper)
    }
}

/// One finite lower/upper bound for a floating-point interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberBound {
    /// No endpoint in this direction.
    Unbounded,
    /// Inclusive finite endpoint.
    Inclusive(f64),
    /// Exclusive finite endpoint.
    Exclusive(f64),
}

/// A validated floating-point interval with finite endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberBounds {
    lower: NumberBound,
    upper: NumberBound,
}

impl NumberBounds {
    /// Return an interval with no lower or upper endpoint.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            lower: NumberBound::Unbounded,
            upper: NumberBound::Unbounded,
        }
    }

    /// Build a validated floating-point interval.
    ///
    /// Returns `None` for non-finite endpoints or empty intervals.
    #[must_use]
    pub fn new(lower: NumberBound, upper: NumberBound) -> Option<Self> {
        if !number_bound_is_finite(lower) || !number_bound_is_finite(upper) {
            return None;
        }

        if let (Some((lower_value, lower_inclusive)), Some((upper_value, upper_inclusive))) =
            (number_bound_value(lower), number_bound_value(upper))
        {
            match lower_value.partial_cmp(&upper_value)? {
                std::cmp::Ordering::Greater => return None,
                std::cmp::Ordering::Equal if !(lower_inclusive && upper_inclusive) => return None,
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {}
            }
        }

        Some(Self { lower, upper })
    }

    /// Return the lower endpoint.
    #[must_use]
    pub const fn lower(self) -> NumberBound {
        self.lower
    }

    /// Return the upper endpoint.
    #[must_use]
    pub const fn upper(self) -> NumberBound {
        self.upper
    }

    /// Return true when `value` is inside this interval.
    #[must_use]
    pub fn contains(self, value: f64) -> bool {
        number_lower_bound_contains(self.lower, value)
            && number_upper_bound_contains(self.upper, value)
    }

    /// Return true when `sub` is wholly contained by this interval.
    #[must_use]
    pub fn contains_bounds(self, sub: Self) -> bool {
        number_lower_bound_is_at_most(self.lower, sub.lower)
            && number_upper_bound_is_at_least(self.upper, sub.upper)
    }
}

fn number_bound_is_finite(bound: NumberBound) -> bool {
    match bound {
        NumberBound::Unbounded => true,
        NumberBound::Inclusive(value) | NumberBound::Exclusive(value) => value.is_finite(),
    }
}

fn number_bound_value(bound: NumberBound) -> Option<(f64, bool)> {
    match bound {
        NumberBound::Unbounded => None,
        NumberBound::Inclusive(value) => Some((value, true)),
        NumberBound::Exclusive(value) => Some((value, false)),
    }
}

fn number_lower_bound_contains(bound: NumberBound, value: f64) -> bool {
    match bound {
        NumberBound::Unbounded => true,
        NumberBound::Inclusive(bound) => value >= bound,
        NumberBound::Exclusive(bound) => value > bound,
    }
}

fn number_upper_bound_contains(bound: NumberBound, value: f64) -> bool {
    match bound {
        NumberBound::Unbounded => true,
        NumberBound::Inclusive(bound) => value <= bound,
        NumberBound::Exclusive(bound) => value < bound,
    }
}

fn number_lower_bound_is_at_most(sup: NumberBound, sub: NumberBound) -> bool {
    match (sup, sub) {
        (NumberBound::Unbounded, _) => true,
        (_, NumberBound::Unbounded) => false,
        (NumberBound::Inclusive(sup), NumberBound::Inclusive(sub))
        | (NumberBound::Inclusive(sup), NumberBound::Exclusive(sub))
        | (NumberBound::Exclusive(sup), NumberBound::Exclusive(sub)) => sub >= sup,
        (NumberBound::Exclusive(sup), NumberBound::Inclusive(sub)) => sub > sup,
    }
}

fn number_upper_bound_is_at_least(sup: NumberBound, sub: NumberBound) -> bool {
    match (sup, sub) {
        (NumberBound::Unbounded, _) => true,
        (_, NumberBound::Unbounded) => false,
        (NumberBound::Inclusive(sup), NumberBound::Inclusive(sub))
        | (NumberBound::Inclusive(sup), NumberBound::Exclusive(sub))
        | (NumberBound::Exclusive(sup), NumberBound::Exclusive(sub)) => sub <= sup,
        (NumberBound::Exclusive(sup), NumberBound::Inclusive(sub)) => sub < sup,
    }
}

/// Inclusive count range with optional upper bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountRange<T> {
    min: T,
    max: Option<T>,
}

impl<T: Copy + Ord> CountRange<T> {
    /// Return an inclusive range with no upper bound.
    #[must_use]
    pub const fn unbounded_from(min: T) -> Self {
        Self { min, max: None }
    }

    /// Build a validated inclusive range.
    ///
    /// Returns `None` when `max` is present and `min > max`.
    #[must_use]
    pub fn new(min: T, max: Option<T>) -> Option<Self> {
        if max.is_some_and(|max| min > max) {
            return None;
        }
        Some(Self { min, max })
    }

    /// Return the inclusive lower endpoint.
    #[must_use]
    pub const fn min(self) -> T {
        self.min
    }

    /// Return the inclusive upper endpoint, if present.
    #[must_use]
    pub const fn max(self) -> Option<T> {
        self.max
    }

    /// Return true when `value` is inside this range.
    #[must_use]
    pub fn contains(self, value: T) -> bool {
        value >= self.min && self.max.is_none_or(|max| value <= max)
    }

    /// Return true when `sub` is wholly contained by this range.
    #[must_use]
    pub fn contains_range(self, sub: Self) -> bool {
        self.min <= sub.min
            && match (self.max, sub.max) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(sup_max), Some(sub_max)) => sub_max <= sup_max,
            }
    }
}

/// Whether a regex pattern can be executed by the internal Rust matcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternSupport {
    /// The pattern compiled into the internal Rust matcher.
    Supported,
    /// The pattern is preserved as source text but cannot be evaluated internally.
    Unsupported,
}

/// A pattern constraint with cached matcher state and its original source text.
#[derive(Clone)]
pub struct PatternConstraint {
    source: String,
    support: PatternSupport,
    matcher: Option<Rc<Regex>>,
}

impl PatternConstraint {
    pub(crate) fn new(source: String) -> Self {
        let matcher = Regex::new(&source).ok().map(Rc::new);
        let support = if matcher.is_some() {
            PatternSupport::Supported
        } else {
            PatternSupport::Unsupported
        };

        Self {
            source,
            support,
            matcher,
        }
    }

    /// Return the original JSON Schema pattern source.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// Return whether the pattern is supported by the internal Rust matcher.
    #[must_use]
    pub const fn support(&self) -> PatternSupport {
        self.support
    }

    /// Return true when the supported internal matcher accepts `candidate`.
    ///
    /// Unsupported patterns return `false`; callers that need Draft 2020-12
    /// validation should use `SchemaDocument::is_valid` instead.
    #[must_use]
    pub fn is_match(&self, candidate: &str) -> bool {
        match self.support {
            PatternSupport::Supported => self
                .matcher
                .as_ref()
                .is_some_and(|regex| regex.is_match(candidate).unwrap_or(false)),
            PatternSupport::Unsupported => false,
        }
    }
}

impl fmt::Debug for PatternConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PatternConstraint")
            .field(&self.source)
            .finish()
    }
}

impl PartialEq for PatternConstraint {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Eq for PatternConstraint {}

impl Hash for PatternConstraint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.hash(state);
    }
}

/// One `patternProperties` entry with both the source pattern and compiled matcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternProperty<Node> {
    /// Compiled pattern and original source text for this entry.
    pub pattern: PatternConstraint,
    /// Schema applied to property values whose names match `pattern`.
    pub schema: Node,
}

impl<Node> PatternProperty<Node> {
    /// Build one pattern-property entry from its pattern and value schema.
    #[must_use]
    pub fn new(pattern: PatternConstraint, schema: Node) -> Self {
        Self { pattern, schema }
    }
}

/// Array `contains` constraints are stored as a single structured value so the
/// count bounds cannot drift out of sync with the subschema itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainsConstraint<Node> {
    /// Schema used to count matching array items.
    pub schema: Node,
    count: CountRange<u64>,
}

impl<Node> ContainsConstraint<Node> {
    /// Build a `contains` constraint with normalized min/max match counts.
    #[must_use]
    pub fn new(schema: Node, count: CountRange<u64>) -> Self {
        Self { schema, count }
    }

    /// Return the inclusive range of allowed matching item counts.
    #[must_use]
    pub const fn count(&self) -> CountRange<u64> {
        self.count
    }
}
