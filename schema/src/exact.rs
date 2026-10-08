//! Exact decimal arithmetic for constraints that cannot use the small-number IR.
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use serde_json::{Map, Value};

/// Parse the decimal value represented by a JSON number, without a binary
/// floating-point division or epsilon comparison.
pub fn decimal(value: &Value) -> Option<BigRational> {
    let text = value.as_number()?.to_string();
    let (mantissa, exponent) = text
        .split_once(['e', 'E'])
        .map_or(Some((text.as_str(), 0_i32)), |(m, e)| {
            Some((m, e.parse().ok()?))
        })?;
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let numerator = format!("{whole}{fraction}").parse::<BigInt>().ok()?;
    let scale = i32::try_from(fraction.len()).ok()?.checked_sub(exponent)?;
    let power = BigInt::from(10_u8).pow(scale.unsigned_abs());
    Some(if scale >= 0 {
        BigRational::new(numerator, power)
    } else {
        BigRational::from_integer(numerator * power)
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Endpoint {
    value: BigRational,
    inclusive: bool,
}

/// Exact numeric assertions. Integer domains are lattices with step one;
/// decimal multipleOf domains are rational lattices anchored at zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactNumber {
    lower: Option<Endpoint>,
    upper: Option<Endpoint>,
    step: Option<BigRational>,
    enumeration: Option<Vec<BigRational>>,
}

impl ExactNumber {
    pub fn from_schema(object: &Map<String, Value>) -> Option<Self> {
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "type"
                    | "minimum"
                    | "maximum"
                    | "exclusiveMinimum"
                    | "exclusiveMaximum"
                    | "multipleOf"
                    | "enum"
            ) && !crate::schema_metadata::is_schema_metadata_key(key)
        }) {
            return None;
        }
        if !matches!(
            object.get("type").and_then(Value::as_str),
            Some("number" | "integer")
        ) {
            return None;
        }
        let endpoint = |inclusive: &str, exclusive: &str, lower: bool| {
            let a = object
                .get(inclusive)
                .and_then(decimal)
                .map(|value| Endpoint {
                    value,
                    inclusive: true,
                });
            let b = object
                .get(exclusive)
                .and_then(decimal)
                .map(|value| Endpoint {
                    value,
                    inclusive: false,
                });
            match (a, b) {
                (Some(a), Some(b)) => Some(
                    if (lower && a.value > b.value) || (!lower && a.value < b.value) {
                        a
                    } else {
                        b
                    },
                ),
                (a, b) => a.or(b),
            }
        };
        let mut step = object.get("multipleOf").and_then(decimal);
        if step
            .as_ref()
            .is_some_and(|step| step <= &BigRational::zero())
        {
            return None;
        }
        if object.get("type").and_then(Value::as_str) == Some("integer") {
            step = Some(step.map_or_else(BigRational::one, |step| {
                BigRational::from_integer(step.numer().clone())
            }));
        }
        let enumeration = object
            .get("enum")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(decimal).collect());
        Some(Self {
            lower: endpoint("minimum", "exclusiveMinimum", true),
            upper: endpoint("maximum", "exclusiveMaximum", false),
            step,
            enumeration,
        })
    }

    pub fn accepts(&self, value: &Value) -> bool {
        decimal(value).is_some_and(|number| self.contains(&number))
    }

    fn contains(&self, value: &BigRational) -> bool {
        self.lower
            .as_ref()
            .is_none_or(|b| value > &b.value || (b.inclusive && value == &b.value))
            && self
                .upper
                .as_ref()
                .is_none_or(|b| value < &b.value || (b.inclusive && value == &b.value))
            && self
                .step
                .as_ref()
                .is_none_or(|step| (value / step).is_integer())
            && self
                .enumeration
                .as_ref()
                .is_none_or(|values| values.contains(value))
    }

    /// A sound inclusion proof; false means this rule did not prove inclusion.
    pub fn is_subset_of(&self, target: &Self) -> bool {
        if let Some(values) = &self.enumeration {
            return values
                .iter()
                .filter(|value| self.contains(value))
                .all(|value| target.contains(value));
        }
        let (lower, upper) = self.effective_bounds();
        if let (Some(lower), Some(upper)) = (&lower, &upper) {
            if lower.value > upper.value
                || (lower.value == upper.value && !(lower.inclusive && upper.inclusive))
            {
                return true;
            }
            if lower.value == upper.value {
                return target.contains(&lower.value);
            }
        }
        if target.enumeration.is_some() {
            return false;
        }
        let lower_fits = target.lower.as_ref().is_none_or(|b| {
            lower.as_ref().is_some_and(|a| {
                a.value > b.value || (a.value == b.value && (b.inclusive || !a.inclusive))
            })
        });
        let upper_fits = target.upper.as_ref().is_none_or(|b| {
            upper.as_ref().is_some_and(|a| {
                a.value < b.value || (a.value == b.value && (b.inclusive || !a.inclusive))
            })
        });
        lower_fits
            && upper_fits
            && target
                .step
                .as_ref()
                .is_none_or(|b| self.step.as_ref().is_some_and(|a| (a / b).is_integer()))
    }

    /// Deterministic, validated seeds for bounded numeric generation.
    pub fn candidates(&self) -> Vec<Value> {
        let mut values = self.enumeration.clone().unwrap_or_default();
        let (lower, upper) = self.effective_bounds();
        let step = self.step.clone().unwrap_or_else(BigRational::one);
        for integer in -4..=4 {
            values.push(BigRational::from_integer(integer.into()) * &step);
        }
        for bound in [&lower, &upper].into_iter().flatten() {
            values.extend([
                bound.value.clone(),
                &bound.value + &step,
                &bound.value - &step,
            ]);
        }
        if let (Some(a), Some(b)) = (lower, upper) {
            values.push((a.value + b.value) / BigRational::from_integer(2.into()));
        }
        values
            .into_iter()
            .filter_map(|value| {
                let result = if value.is_integer() {
                    let integer = value.to_integer();
                    integer
                        .to_i64()
                        .map(Value::from)
                        .or_else(|| integer.to_u64().map(Value::from))
                        .or_else(|| {
                            value
                                .to_f64()
                                .and_then(serde_json::Number::from_f64)
                                .map(Value::Number)
                        })
                } else {
                    value
                        .to_f64()
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                }?;
                self.accepts(&result).then_some(result)
            })
            .collect()
    }

    fn effective_bounds(&self) -> (Option<Endpoint>, Option<Endpoint>) {
        let Some(step) = &self.step else {
            return (self.lower.clone(), self.upper.clone());
        };
        let round = |bound: &Endpoint, lower: bool| {
            let quotient = &bound.value / step;
            let mut integer = if lower {
                quotient.ceil().to_integer()
            } else {
                quotient.floor().to_integer()
            };
            if !bound.inclusive && quotient.is_integer() {
                integer += if lower { BigInt::one() } else { -BigInt::one() };
            }
            Endpoint {
                value: BigRational::from_integer(integer) * step,
                inclusive: true,
            }
        };
        (
            self.lower.as_ref().map(|b| round(b, true)),
            self.upper.as_ref().map(|b| round(b, false)),
        )
    }
}

pub(crate) fn needs_exact_number(object: &Map<String, Value>) -> bool {
    if object
        .get("multipleOf")
        .and_then(Value::as_f64)
        .is_some_and(|v| v.fract() != 0.0)
    {
        return true;
    }
    [
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
    ]
    .into_iter()
    .any(|key| {
        object
            .get(key)
            .and_then(Value::as_f64)
            .is_some_and(|value| value.abs() > 9_007_199_254_740_991.0)
    })
}

/// Install exact numeric predicates in the validation backend too, so a proof
/// and its counterexample validator use the same arithmetic.
pub(crate) fn configure(
    mut options: jsonschema::ValidationOptions,
) -> jsonschema::ValidationOptions {
    for keyword in [
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
    ] {
        options = options.with_keyword(keyword, move |_, value, _| {
            let bound = decimal(value).ok_or_else(|| {
                jsonschema::ValidationError::custom("expected a finite JSON number")
            })?;
            let small = bound
                .to_f64()
                .filter(|value| value.abs() <= 9_007_199_254_740_991.0);
            Ok(Box::new(NumericKeyword {
                keyword,
                bound,
                small,
            }))
        });
    }
    options
}

struct NumericKeyword {
    keyword: &'static str,
    bound: BigRational,
    small: Option<f64>,
}
impl jsonschema::Keyword for NumericKeyword {
    fn is_valid(&self, instance: &Value) -> bool {
        if !instance.is_number() {
            return true;
        }
        if let Some(bound) = self.small {
            if self.keyword == "multipleOf" && bound > 0.0 && bound.fract() == 0.0 {
                if let Some(value) = instance.as_i64() {
                    return i128::from(value) % (bound as i128) == 0;
                }
                if let Some(value) = instance.as_u64() {
                    return u128::from(value) % (bound as u128) == 0;
                }
            }
            if self.keyword != "multipleOf"
                && let Some(value) = instance
                    .as_f64()
                    .filter(|value| value.abs() <= 9_007_199_254_740_991.0)
            {
                return match self.keyword {
                    "minimum" => value >= bound,
                    "maximum" => value <= bound,
                    "exclusiveMinimum" => value > bound,
                    "exclusiveMaximum" => value < bound,
                    _ => unreachable!(),
                };
            }
        }
        let Some(value) = decimal(instance) else {
            return false;
        };
        match self.keyword {
            "minimum" => value >= self.bound,
            "maximum" => value <= self.bound,
            "exclusiveMinimum" => value > self.bound,
            "exclusiveMaximum" => value < self.bound,
            "multipleOf" => self.bound > BigRational::zero() && (value / &self.bound).is_integer(),
            _ => unreachable!(),
        }
    }
    fn validate<'i>(&self, instance: &'i Value) -> Result<(), jsonschema::ValidationError<'i>> {
        if self.is_valid(instance) {
            Ok(())
        } else {
            Err(jsonschema::ValidationError::custom(format!(
                "exact {} constraint failed",
                self.keyword
            )))
        }
    }
}
