//! Exact decimal values without expanding powers of ten.
//!
//! The coefficient and exponent use space proportional to their JSON spelling.
//! Even `1e2147483648` can be compared, tested for integrality, and used in
//! divisibility checks without constructing a multi-gigabyte integer.
use num_bigint::{BigInt, BigUint};
use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Decimal {
    negative: bool,
    digits: String,
    exponent: BigInt,
}
impl Decimal {
    pub fn from_value(value: &Value) -> Option<Self> {
        value.as_number().map(Self::from_number)
    }
    pub fn from_number(value: &Number) -> Self {
        Self::parse_number(&value.to_string())
    }
    pub fn from_i64(value: i64) -> Self {
        Self::parse_number(&value.to_string())
    }
    pub fn from_u64(value: u64) -> Self {
        Self::parse_number(&value.to_string())
    }
    fn parse_number(text: &str) -> Self {
        let negative = text.starts_with('-');
        let text = text.strip_prefix('-').unwrap_or(text);
        let (mantissa, exponent) = text.split_once(['e', 'E']).unwrap_or((text, "0"));
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let digits = format!("{whole}{fraction}");
        let digits = digits.trim_start_matches('0');
        if digits.is_empty() {
            return Self {
                negative: false,
                digits: "0".into(),
                exponent: BigInt::zero(),
            };
        }
        let normalized = digits.trim_end_matches('0');
        Self {
            negative,
            digits: normalized.into(),
            exponent: exponent
                .parse::<BigInt>()
                .expect("JSON exponent is an integer")
                - BigInt::from(fraction.len())
                + BigInt::from(digits.len() - normalized.len()),
        }
    }
    pub fn is_zero(&self) -> bool {
        self.digits == "0"
    }
    pub fn is_integer(&self) -> bool {
        self.exponent >= BigInt::zero()
    }
    pub fn is_positive(&self) -> bool {
        !self.negative && !self.is_zero()
    }
    pub fn to_u64(&self) -> Option<u64> {
        if self.negative {
            return None;
        }
        let exponent = self.exponent.to_u32()?;
        self.digits
            .parse::<u64>()
            .ok()?
            .checked_mul(10_u64.checked_pow(exponent)?)
    }
    pub fn to_i128(&self) -> Option<i128> {
        let exponent = self.exponent.to_u32()?;
        let coefficient = if self.negative {
            format!("-{}", self.digits)
        } else {
            self.digits.clone()
        };
        coefficient
            .parse::<i128>()
            .ok()?
            .checked_mul(10_i128.checked_pow(exponent)?)
    }
    /// Exact divisibility; exponentiation is modular, so its output is bounded
    /// by the divisor coefficient rather than by the exponent's magnitude.
    pub fn is_multiple_of(&self, divisor: &Self) -> bool {
        if !divisor.is_positive() {
            return false;
        }
        if self.is_zero() {
            return true;
        }
        let shift = &self.exponent - &divisor.exponent;
        if shift < BigInt::zero() {
            return false;
        }
        let numerator: BigUint = self.digits.parse().expect("decimal coefficient");
        let denominator: BigUint = divisor.digits.parse().expect("decimal coefficient");
        let factor = BigUint::from(10_u8).modpow(
            &shift.to_biguint().expect("nonnegative shift"),
            &denominator,
        );
        (numerator * factor % denominator).is_zero()
    }
    /// The proof engine uses rational lattices. Refuse expansion beyond its
    /// bounded representation; validation above never needs this expansion.
    pub(crate) fn rational(&self) -> Option<BigRational> {
        let exponent = self.exponent.to_i32()?;
        if exponent.unsigned_abs() > 10_000 {
            return None;
        }
        let mut coefficient: BigInt = self.digits.parse().ok()?;
        if self.negative {
            coefficient = -coefficient;
        }
        let power = BigInt::from(10_u8).pow(exponent.unsigned_abs());
        Some(if exponent < 0 {
            BigRational::new(coefficient, power)
        } else {
            BigRational::from_integer(coefficient * power)
        })
    }
}
impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.negative != other.negative {
            return other.negative.cmp(&self.negative);
        }
        let order = if self.is_zero() || other.is_zero() {
            other.is_zero().cmp(&self.is_zero())
        } else {
            (&self.exponent + BigInt::from(self.digits.len()))
                .cmp(&(&other.exponent + BigInt::from(other.digits.len())))
                .then_with(|| {
                    let length = self.digits.len().max(other.digits.len());
                    self.digits
                        .bytes()
                        .chain(std::iter::repeat(b'0'))
                        .take(length)
                        .cmp(
                            other
                                .digits
                                .bytes()
                                .chain(std::iter::repeat(b'0'))
                                .take(length),
                        )
                })
        };
        if self.negative {
            order.reverse()
        } else {
            order
        }
    }
}
impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl TryFrom<String> for Decimal {
    type Error = serde_json::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let number = value.parse::<Number>()?;
        Ok(Self::from_number(&number))
    }
}
impl From<Decimal> for String {
    fn from(value: Decimal) -> Self {
        format!(
            "{}{}e{}",
            if value.negative { "-" } else { "" },
            value.digits,
            value.exponent
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn number(text: &str) -> Decimal {
        Decimal::try_from(text.to_owned()).unwrap()
    }
    #[test]
    fn extreme_exponents_remain_exact_and_compact() {
        for exponent in ["10000", "10001", "2147483648", "999999999999999999999999"] {
            let tiny = number(&format!("1e-{exponent}"));
            let huge = number(&format!("1e{exponent}"));
            assert!(tiny > number("0") && tiny < number("1"));
            assert!(huge > number("1"));
            assert!(!tiny.is_integer());
            assert!(huge.is_integer());
            assert!(huge.is_multiple_of(&number("2")));
            assert!(!huge.is_multiple_of(&number("3")));
            assert!(!tiny.is_multiple_of(&number("1")));
            assert_eq!(
                tiny,
                number(&format!("10e-{}", number_exponent_plus_one(exponent)))
            );
        }
    }
    fn number_exponent_plus_one(text: &str) -> BigInt {
        text.parse::<BigInt>().unwrap() + 1
    }
    #[test]
    fn decimal_semantics_match_bounded_rationals() {
        // Independent rational arithmetic checks signs, normalization, ordering
        // and divisibility over a deterministic cross product.
        let values: Vec<_> = (-12..=12)
            .flat_map(|n| (-4..=4).map(move |e| number(&format!("{n}e{e}"))))
            .collect();
        for a in &values {
            for b in &values {
                let x = a.rational().unwrap();
                let y = b.rational().unwrap();
                assert_eq!(a.cmp(b), x.cmp(&y));
                assert_eq!(a.is_integer(), x.is_integer());
                assert_eq!(
                    a.is_multiple_of(b),
                    y > BigRational::zero() && (&x / &y).is_integer()
                );
            }
        }
    }
}
