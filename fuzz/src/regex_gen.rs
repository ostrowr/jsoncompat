//! Generate random strings that match a given regular expression.
//!
//! Parses the pattern with [`fancy_regex::Expr`] and recursively walks the AST
//! to produce a matching string.  Character classes and other constructs that
//! `fancy-regex` delegates to the standard `regex` crate are handled via
//! [`regex_syntax`]'s HIR.

use fancy_regex::Expr;
use rand::{Rng, RngExt};
use regex_syntax::hir::{Class, Hir, HirKind};

/// Maximum number of repetitions chosen for unbounded quantifiers (`*`, `+`).
const MAX_REPEAT: usize = 6;

/// Generate a random string matching `pattern`.
///
/// Returns `None` when the pattern cannot be parsed or contains unsupported
/// constructs (e.g. ECMA-262 `\c` control escapes).
pub fn generate_matching_string(pattern: &str, rng: &mut impl Rng) -> Option<String> {
    let tree = Expr::parse_tree(pattern).ok()?;
    let mut generator = RegexGenerator { rng };
    Some(generator.gen_expr(&tree.expr))
}

struct RegexGenerator<'a, R> {
    rng: &'a mut R,
}

impl<R: Rng> RegexGenerator<'_, R> {
    fn gen_expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Empty
            | Expr::StartText
            | Expr::EndText
            | Expr::StartLine
            | Expr::EndLine
            | Expr::KeepOut
            | Expr::ContinueFromPreviousMatchEnd
            | Expr::BackrefExistsCondition(_) => String::new(),

            Expr::Literal { val, casei } => {
                if *casei {
                    val.chars()
                        .map(|c| {
                            if self.rng.random_bool(0.5) {
                                flip_case(c)
                            } else {
                                c
                            }
                        })
                        .collect()
                } else {
                    val.clone()
                }
            }

            Expr::Any { newline } => {
                if *newline {
                    // Any character including newline — pick from full printable
                    // ASCII (0x20–0x7E, 95 chars) or \n (slot 95).
                    let n = self.rng.random_range(0u8..96);
                    let byte = if n == 95 { b'\n' } else { 0x20 + n };
                    char::from(byte).to_string()
                } else {
                    // Printable ASCII, no newline.
                    let c = self.rng.random_range(0x20u8..=0x7E);
                    char::from(c).to_string()
                }
            }

            Expr::Concat(exprs) => {
                let mut out = String::new();
                for e in exprs {
                    out.push_str(&self.gen_expr(e));
                }
                out
            }

            Expr::Alt(exprs) if !exprs.is_empty() => {
                let idx = self.rng.random_range(0..exprs.len());
                self.gen_expr(&exprs[idx])
            }

            Expr::Repeat { child, lo, hi, .. } => {
                let upper = if *hi == usize::MAX {
                    lo.saturating_add(MAX_REPEAT)
                } else {
                    *hi
                };
                let count = if *lo <= upper {
                    self.rng.random_range(*lo..=upper)
                } else {
                    *lo
                };
                let mut out = String::new();
                for _ in 0..count {
                    out.push_str(&self.gen_expr(child));
                }
                out
            }

            Expr::Group(inner) => self.gen_expr(inner),

            Expr::Delegate { inner, casei, .. } => self.gen_from_delegate(inner, *casei),

            Expr::Backref(n) => {
                log::warn!(
                    "backreference \\{n} is not yet supported; generated string may not match the pattern"
                );
                String::new()
            }

            Expr::AtomicGroup(inner) => self.gen_expr(inner),

            Expr::Conditional {
                true_branch,
                false_branch,
                ..
            } => {
                if self.rng.random_bool(0.5) {
                    self.gen_expr(true_branch)
                } else {
                    self.gen_expr(false_branch)
                }
            }

            _ => String::new(),
        }
    }

    fn gen_from_delegate(&mut self, inner: &str, is_case_insensitive: bool) -> String {
        // First try parsing with unicode disabled to match ECMA-262 semantics
        // (where \d = [0-9], \w = [A-Za-z0-9_], etc.).  Fall back to unicode
        // mode for patterns that require it (e.g. \p{Letter}).
        let hir = regex_syntax::ParserBuilder::new()
            .unicode(false)
            .utf8(false)
            .build()
            .parse(inner)
            .ok()
            .or_else(|| regex_syntax::parse(inner).ok());
        let Some(hir) = hir else {
            return String::new();
        };
        let value = self.gen_from_hir(&hir);
        if is_case_insensitive {
            value
                .chars()
                .map(|c| {
                    if self.rng.random_bool(0.5) {
                        flip_case(c)
                    } else {
                        c
                    }
                })
                .collect()
        } else {
            value
        }
    }

    fn gen_from_hir(&mut self, hir: &Hir) -> String {
        match hir.kind() {
            HirKind::Empty => String::new(),

            HirKind::Literal(lit) => String::from_utf8_lossy(&lit.0).into_owned(),

            HirKind::Class(cls) => match cls {
                Class::Unicode(u) => sample_unicode_class(u, self.rng)
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
                Class::Bytes(b) => {
                    // Restrict to ASCII to avoid generating bytes that may
                    // not match the validator's unicode-aware interpretation.
                    sample_bytes_class_ascii(b, self.rng)
                        .map(|byte| (byte as char).to_string())
                        .unwrap_or_default()
                }
            },

            HirKind::Repetition(rep) => {
                let lo = rep.min as usize;
                let hi = rep.max.map(|m| m as usize).unwrap_or(lo + MAX_REPEAT);
                let upper = hi.min(lo.saturating_add(MAX_REPEAT));
                let count = if lo <= upper {
                    self.rng.random_range(lo..=upper)
                } else {
                    lo
                };
                let mut out = String::new();
                for _ in 0..count {
                    out.push_str(&self.gen_from_hir(&rep.sub));
                }
                out
            }

            HirKind::Capture(cap) => self.gen_from_hir(&cap.sub),

            HirKind::Concat(subs) => {
                let mut out = String::new();
                for s in subs {
                    out.push_str(&self.gen_from_hir(s));
                }
                out
            }

            HirKind::Alternation(subs) if !subs.is_empty() => {
                let idx = self.rng.random_range(0..subs.len());
                self.gen_from_hir(&subs[idx])
            }

            HirKind::Look(_) => String::new(),

            _ => String::new(),
        }
    }
}

fn flip_case(c: char) -> char {
    if c.is_uppercase() {
        c.to_lowercase().next().unwrap_or(c)
    } else if c.is_lowercase() {
        c.to_uppercase().next().unwrap_or(c)
    } else {
        c
    }
}

/// Pick a random character from a `ClassUnicode`.
fn sample_unicode_class(cls: &regex_syntax::hir::ClassUnicode, rng: &mut impl Rng) -> Option<char> {
    let ranges = cls.ranges();
    if ranges.is_empty() {
        return None;
    }
    let range = &ranges[rng.random_range(0..ranges.len())];
    let offset = rng.random_range(0..range.len()) as u32;
    char::from_u32(u32::from(range.start()) + offset)
}

/// Pick a random byte from a `ClassBytes`, restricted to ASCII (0x00..=0x7F)
/// to avoid generating bytes that the unicode-aware validator would reject.
fn sample_bytes_class_ascii(cls: &regex_syntax::hir::ClassBytes, rng: &mut impl Rng) -> Option<u8> {
    // Clamp each range to the ASCII region.
    let ascii_ranges: Vec<(u8, u8)> = cls
        .ranges()
        .iter()
        .filter_map(|r| {
            let lo = r.start();
            let hi = r.end().min(0x7F);
            (lo <= hi).then_some((lo, hi))
        })
        .collect();

    if ascii_ranges.is_empty() {
        return None;
    }

    let &(lo, hi) = &ascii_ranges[rng.random_range(0..ascii_ranges.len())];
    Some(rng.random_range(lo..=hi))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fancy_regex::Regex;
    use rand::{SeedableRng, rngs::StdRng};

    /// Assert that `generate_matching_string` produces strings that pass the
    /// regex for every iteration.
    fn check_pattern(pattern: &str, iterations: usize) {
        let re = Regex::new(pattern).expect("regex should compile");
        let mut rng = StdRng::seed_from_u64(42);
        for i in 0..iterations {
            let s = generate_matching_string(pattern, &mut rng)
                .unwrap_or_else(|| panic!("should generate for {pattern:?}"));
            assert!(
                re.is_match(&s).unwrap_or(false),
                "Iteration {i}: pattern {pattern:?} not matched by {s:?}"
            );
        }
    }

    #[test]
    fn literal() {
        check_pattern("abc", 100);
    }

    #[test]
    fn anchored_literal() {
        check_pattern("^abc$", 100);
    }

    #[test]
    fn star() {
        check_pattern("^a*$", 100);
    }

    #[test]
    fn plus() {
        check_pattern("^a+$", 100);
    }

    #[test]
    fn question_mark() {
        check_pattern("^ab?c$", 100);
    }

    #[test]
    fn repeat_range() {
        check_pattern("^a{2,5}$", 100);
    }

    #[test]
    fn repeat_exact() {
        check_pattern("^x{3}$", 100);
    }

    #[test]
    fn alternation() {
        check_pattern("^(cat|dog|fish)$", 100);
    }

    #[test]
    fn char_class() {
        check_pattern("^[a-z]+$", 100);
    }

    #[test]
    fn char_class_mixed() {
        check_pattern("^[a-zA-Z0-9_]+$", 100);
    }

    #[test]
    fn negated_class() {
        check_pattern("^[^0-9]+$", 50);
    }

    #[test]
    fn digit_shorthand() {
        check_pattern("^\\d{3}$", 100);
    }

    #[test]
    fn word_shorthand() {
        check_pattern("^\\w+$", 100);
    }

    #[test]
    fn space_shorthand() {
        check_pattern("^\\s+$", 100);
    }

    #[test]
    fn non_digit() {
        check_pattern("^\\D+$", 50);
    }

    #[test]
    fn non_word() {
        check_pattern("^\\W+$", 50);
    }

    #[test]
    fn non_space() {
        check_pattern("^\\S+$", 50);
    }

    #[test]
    fn dot() {
        check_pattern("^.{5}$", 100);
    }

    #[test]
    fn backref_not_yet_supported() {
        // Backreferences produce an empty string for now, so the generated
        // value won't necessarily match the pattern.  Just verify we don't panic.
        let mut rng = StdRng::seed_from_u64(42);
        assert!(generate_matching_string("^(a|b)\\1$", &mut rng).is_some());
    }

    #[test]
    fn nested_groups() {
        check_pattern("^((a|b)c)+$", 100);
    }

    #[test]
    fn unicode_property() {
        check_pattern("^\\p{Letter}+$", 50);
    }

    #[test]
    fn unicode_emoji_literal() {
        check_pattern("^🐲*$", 100);
    }

    #[test]
    fn unicode_digit_property() {
        check_pattern("^\\p{digit}+$", 50);
    }

    #[test]
    fn identifier_pattern() {
        check_pattern("^[a-zA-Z][a-zA-Z0-9_]*$", 100);
    }

    #[test]
    fn email_like() {
        check_pattern("^[a-z]+@[a-z]+\\.[a-z]{2,4}$", 100);
    }

    #[test]
    fn digit_sequence() {
        check_pattern("^\\d+$", 100);
    }

    #[test]
    fn empty_pattern() {
        check_pattern("", 10);
    }

    #[test]
    fn unanchored_pattern() {
        check_pattern("a+", 100);
    }

    #[test]
    fn tab_escape() {
        check_pattern("^\\t$", 100);
    }

    #[test]
    fn unsupported_pattern_returns_none() {
        // \cC is an ECMA-262 control escape that fancy-regex cannot parse.
        let mut rng = StdRng::seed_from_u64(42);
        assert!(generate_matching_string("^\\cC$", &mut rng).is_none());
    }

    #[test]
    fn unicode_letter_cole() {
        check_pattern("\\p{Letter}cole", 100);
    }
}
