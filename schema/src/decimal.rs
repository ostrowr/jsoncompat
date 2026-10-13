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
        // Prepared artifacts store exact decimals as strings. Validate the JSON
        // number grammar without passing through Number: its default f64 path
        // would round exponent notation even for coefficients that fit in u64.
        if !is_json_number(&value) {
            return Err(serde::de::Error::custom("invalid JSON decimal"));
        }
        Ok(Self::parse_number(&value))
    }
}

fn is_json_number(text: &str) -> bool {
    fn digits(bytes: &mut &[u8]) -> bool {
        let count = bytes
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        *bytes = &bytes[count..];
        count != 0
    }

    let mut bytes = text.as_bytes();
    bytes = bytes.strip_prefix(b"-").unwrap_or(bytes);
    match bytes.first() {
        Some(b'0') => bytes = &bytes[1..],
        Some(b'1'..=b'9') => {
            digits(&mut bytes);
        }
        _ => return false,
    }
    if let Some(rest) = bytes.strip_prefix(b".") {
        bytes = rest;
        if !digits(&mut bytes) {
            return false;
        }
    }
    if matches!(bytes.first(), Some(b'e' | b'E')) {
        bytes = &bytes[1..];
        if matches!(bytes.first(), Some(b'+' | b'-')) {
            bytes = &bytes[1..];
        }
        if !digits(&mut bytes) {
            return false;
        }
    }
    bytes.is_empty()
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
    fn serialization_preserves_exact_values_in_both_precision_modes() {
        for value in [
            Decimal::from_u64(10_000_000_000_000_001),
            Decimal::from_u64(u64::MAX),
            Decimal::from_i64(i64::MIN),
            number("0.10000000000000001"),
            number("-123456789012345678901234567890.125"),
            number("1e2147483648"),
            number("1e-2147483648"),
        ] {
            let encoded = serde_json::to_string(&value).unwrap();
            let decoded: Decimal = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, value, "{encoded}");
        }
        assert_eq!(
            number("1.0000000000000001e16"),
            Decimal::from_u64(10_000_000_000_000_001)
        );
        assert_eq!(number("-0.00E+999"), number("0"));
        assert_eq!(number("12.30E-2"), number("0.123"));
    }

    #[test]
    fn deserialization_rejects_invalid_number_syntax() {
        for text in [
            "", " ", " 1", "1 ", "-", "+1", "01", "-01", ".1", "1.", "1e", "1e+", "1e-", "--1",
            "1e1e1", "NaN", "Infinity", "0x10", "1_0", "[1]", "null", "\"1\"", "١", "1e١",
        ] {
            let encoded = serde_json::to_string(text).unwrap();
            assert!(
                serde_json::from_str::<Decimal>(&encoded).is_err(),
                "{text:?}"
            );
        }
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
