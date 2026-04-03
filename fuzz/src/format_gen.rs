//! Generate random strings that satisfy JSON Schema `format` constraints.
//!
//! Uses the `fake` crate for most formats.  Unknown formats return `None`
//! so the caller can fall back to generic string generation.

use fake::Fake;
use fake::faker::chrono::raw::*;
use fake::faker::internet::raw::*;
use fake::locales::EN;
use rand::{Rng, RngExt};

/// Generate a random string for the given JSON Schema `format` keyword.
///
/// Returns `None` for unrecognised formats.
pub fn generate_for_format(format: &str, rng: &mut impl Rng) -> Option<String> {
    match format {
        "date" => {
            let d: chrono::NaiveDate = Date(EN).fake_with_rng(rng);
            Some(d.format("%Y-%m-%d").to_string())
        }
        "date-time" => {
            let dt: chrono::DateTime<chrono::Utc> = DateTime(EN).fake_with_rng(rng);
            Some(dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        }
        "time" => {
            let t: chrono::NaiveTime = Time(EN).fake_with_rng(rng);
            Some(t.format("%H:%M:%S").to_string())
        }
        "email" | "idn-email" => {
            let e: String = SafeEmail(EN).fake_with_rng(rng);
            Some(e)
        }
        "uri" | "iri" | "uri-reference" | "iri-reference" => Some(gen_uri(rng)),
        "uuid" => {
            let u: uuid::Uuid = fake::uuid::UUIDv4.fake_with_rng(rng);
            Some(u.to_string())
        }
        "ipv4" => {
            let ip: String = IPv4(EN).fake_with_rng(rng);
            Some(ip)
        }
        "ipv6" => {
            let ip: String = IPv6(EN).fake_with_rng(rng);
            Some(ip)
        }
        "hostname" | "idn-hostname" => Some(gen_hostname(rng)),
        _ => None,
    }
}

fn gen_uri(rng: &mut impl Rng) -> String {
    let host = random_alpha(rng, 3..8);
    let tld = random_alpha(rng, 2..4);
    let path = random_alpha(rng, 2..6);
    format!("https://{host}.{tld}/{path}")
}

fn gen_hostname(rng: &mut impl Rng) -> String {
    let name = random_alpha(rng, 3..10);
    let tld = random_alpha(rng, 2..4);
    format!("{name}.{tld}")
}

fn random_alpha(rng: &mut impl Rng, len_range: std::ops::Range<usize>) -> String {
    let len = rng.random_range(len_range);
    (0..len)
        .map(|_| (b'a' + rng.random_range(0..26u8)) as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn date_is_valid() {
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..1000 {
            let d = generate_for_format("date", &mut rng).unwrap();
            let parts: Vec<&str> = d.split('-').collect();
            assert_eq!(parts.len(), 3, "bad date: {d}");
            let _y: u16 = parts[0].parse().unwrap();
            let m: u8 = parts[1].parse().unwrap();
            let _day: u8 = parts[2].parse().unwrap();
            assert!((1..=12).contains(&m));
        }
    }

    #[test]
    fn date_time_format() {
        let mut rng = StdRng::seed_from_u64(42);
        let dt = generate_for_format("date-time", &mut rng).unwrap();
        assert!(dt.contains('T'));
        assert!(dt.ends_with('Z'));
    }

    #[test]
    fn email_format() {
        let mut rng = StdRng::seed_from_u64(42);
        let e = generate_for_format("email", &mut rng).unwrap();
        assert!(e.contains('@'));
        assert!(e.contains('.'));
    }

    #[test]
    fn uuid_format() {
        let mut rng = StdRng::seed_from_u64(42);
        let u = generate_for_format("uuid", &mut rng).unwrap();
        assert_eq!(u.len(), 36);
        assert_eq!(u.chars().filter(|c| *c == '-').count(), 4);
    }

    #[test]
    fn ipv4_format() {
        let mut rng = StdRng::seed_from_u64(42);
        let ip = generate_for_format("ipv4", &mut rng).unwrap();
        let parts: Vec<&str> = ip.split('.').collect();
        assert_eq!(parts.len(), 4);
    }

    #[test]
    fn ipv6_format() {
        let mut rng = StdRng::seed_from_u64(42);
        let ip = generate_for_format("ipv6", &mut rng).unwrap();
        assert!(ip.contains(':'));
    }

    #[test]
    fn unknown_format_returns_none() {
        let mut rng = StdRng::seed_from_u64(42);
        assert!(generate_for_format("custom-thing", &mut rng).is_none());
    }

    #[test]
    fn all_supported_formats_return_some() {
        let mut rng = StdRng::seed_from_u64(42);
        for f in &[
            "date",
            "date-time",
            "time",
            "email",
            "idn-email",
            "uri",
            "iri",
            "uri-reference",
            "iri-reference",
            "uuid",
            "ipv4",
            "ipv6",
            "hostname",
            "idn-hostname",
        ] {
            assert!(
                generate_for_format(f, &mut rng).is_some(),
                "format {f} should generate"
            );
        }
    }
}
