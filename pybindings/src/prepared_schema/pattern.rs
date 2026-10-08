//! Precompiled regular automata and composition for zero-width lookarounds.
//!
//! Both byte orders are stored so a prepared Python module is portable. The
//! safe DFA loader checks transition targets; loading never parses a pattern.

use fancy_regex::{Assertion, Expr, LookAround};
use regex_automata::{
    Anchored, Input, MatchKind,
    dfa::{Automaton, dense, sparse},
};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize)]
pub(super) struct PortableDfa {
    little: Vec<u8>,
    big: Vec<u8>,
    #[serde(skip)]
    native: sparse::DFA<Vec<u8>>,
}

impl<'de> Deserialize<'de> for PortableDfa {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            little: Vec<u8>,
            big: Vec<u8>,
        }
        let Wire { little, big } = Wire::deserialize(deserializer)?;
        let bytes = if cfg!(target_endian = "little") {
            &little
        } else {
            &big
        };
        let (dfa, consumed) = sparse::DFA::from_bytes(bytes).map_err(serde::de::Error::custom)?;
        if consumed != bytes.len() {
            return Err(serde::de::Error::custom("trailing bytes in prepared regex"));
        }
        let native = dfa.to_owned();
        Ok(Self {
            little,
            big,
            native,
        })
    }
}

impl PortableDfa {
    fn compile(pattern: &str) -> Result<Self, String> {
        let dfa = dense::Builder::new()
            .configure(
                dense::Config::new()
                    .match_kind(MatchKind::All)
                    .dfa_size_limit(Some(16 * 1024 * 1024)),
            )
            .build(pattern)
            .map_err(|error| error.to_string())?
            .to_sparse()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            little: dfa.to_bytes_little_endian(),
            big: dfa.to_bytes_big_endian(),
            native: dfa,
        })
    }

    fn ends(&self, text: &str, start: usize, budget: &mut usize) -> Vec<usize> {
        let Ok(mut state) = self.native.start_state_forward(
            &Input::new(text)
                .span(start..text.len())
                .anchored(Anchored::Yes),
        ) else {
            return Vec::new();
        };
        let mut ends = Vec::new();
        for (offset, byte) in text.as_bytes()[start..].iter().enumerate() {
            if !spend(budget) {
                return Vec::new();
            }
            state = self.native.next_state(state, *byte);
            if self.native.is_match_state(state) {
                ends.push(start + offset);
            }
            if self.native.is_dead_state(state) || self.native.is_quit_state(state) {
                return ends;
            }
        }
        state = self.native.next_eoi_state(state);
        if self.native.is_match_state(state) {
            ends.push(text.len());
        }
        ends
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Pattern {
    Dfa(Box<PortableDfa>),
    Sequence(Vec<Pattern>),
    Alternative(Vec<Pattern>),
    Look {
        child: Box<Pattern>,
        behind: bool,
        negative: bool,
    },
    Repeat {
        child: Box<Pattern>,
        min: usize,
        max: usize,
    },
}

impl Pattern {
    pub(super) fn compile(pattern: &str) -> Result<Self, String> {
        let translated = translate(pattern);
        if let Ok(dfa) = PortableDfa::compile(&translated) {
            return Ok(Self::Dfa(Box::new(dfa)));
        }
        let tree = Expr::parse_tree(&translated).map_err(|error| error.to_string())?;
        Self::from_expr(&tree.expr)
    }

    fn from_expr(expr: &Expr) -> Result<Self, String> {
        match expr {
            Expr::LookAround(child, kind) => Ok(Self::Look {
                child: Box::new(Self::from_expr(child)?),
                behind: matches!(kind, LookAround::LookBehind | LookAround::LookBehindNeg),
                negative: matches!(kind, LookAround::LookAheadNeg | LookAround::LookBehindNeg),
            }),
            Expr::Concat(children) => Ok(Self::Sequence(
                children
                    .iter()
                    .map(Self::from_expr)
                    .collect::<Result<_, _>>()?,
            )),
            Expr::Alt(children) => Ok(Self::Alternative(
                children
                    .iter()
                    .map(Self::from_expr)
                    .collect::<Result<_, _>>()?,
            )),
            Expr::Group(child) => Self::from_expr(child),
            Expr::Repeat { child, lo, hi, .. } => Ok(Self::Repeat {
                child: Box::new(Self::from_expr(child)?),
                min: *lo,
                max: *hi,
            }),
            Expr::Empty
            | Expr::Any { .. }
            | Expr::Literal { .. }
            | Expr::Delegate { .. }
            | Expr::Assertion(
                Assertion::StartText
                | Assertion::EndText
                | Assertion::StartLine { .. }
                | Assertion::EndLine { .. },
            ) => {
                let mut pattern = String::new();
                expr.to_str(&mut pattern, 0);
                PortableDfa::compile(&pattern).map(|dfa| Self::Dfa(Box::new(dfa)))
            }
            _ => Err(
                "regex backreferences, atomic groups, and subroutine calls cannot be prepared yet"
                    .into(),
            ),
        }
    }

    pub(super) fn is_match(&self, text: &str) -> bool {
        if let Self::Dfa(dfa) = self {
            return dfa
                .native
                .try_search_fwd(&Input::new(text))
                .is_ok_and(|found| found.is_some());
        }
        let mut budget = 1_000_000;
        for start in text
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(text.len()))
        {
            if !self.ends(text, start, &mut budget).is_empty() {
                return true;
            }
            if budget == 0 {
                return false;
            }
        }
        false
    }

    fn ends(&self, text: &str, start: usize, budget: &mut usize) -> Vec<usize> {
        if !spend(budget) {
            return Vec::new();
        }
        match self {
            Self::Dfa(dfa) => dfa.ends(text, start, budget),
            Self::Sequence(children) => {
                let mut positions = vec![start];
                for child in children {
                    positions = positions
                        .into_iter()
                        .flat_map(|position| child.ends(text, position, budget))
                        .collect();
                    positions.sort_unstable();
                    positions.dedup();
                    if positions.is_empty() {
                        break;
                    }
                }
                positions
            }
            Self::Alternative(children) => children
                .iter()
                .flat_map(|child| child.ends(text, start, budget))
                .collect(),
            Self::Look {
                child,
                behind,
                negative,
            } => {
                let matched = if *behind {
                    text.char_indices()
                        .map(|(index, _)| index)
                        .chain(std::iter::once(text.len()))
                        .take_while(|index| *index <= start)
                        .any(|index| child.ends(text, index, budget).contains(&start))
                } else {
                    !child.ends(text, start, budget).is_empty()
                };
                if matched != *negative && *budget != 0 {
                    vec![start]
                } else {
                    Vec::new()
                }
            }
            Self::Repeat { child, min, max } => {
                let mut positions = vec![start];
                let mut accepted = if *min == 0 {
                    positions.clone()
                } else {
                    Vec::new()
                };
                for count in 1..=*max {
                    if !spend(budget) {
                        return Vec::new();
                    }
                    let mut next: Vec<_> = positions
                        .iter()
                        .flat_map(|position| child.ends(text, *position, budget))
                        .collect();
                    next.sort_unstable();
                    next.dedup();
                    if count >= *min {
                        accepted.extend(&next);
                    }
                    if next == positions {
                        // Additional empty repetitions leave the same positions.
                        if *min <= *max {
                            accepted.extend(&next);
                        }
                        break;
                    }
                    if next.is_empty() {
                        break;
                    }
                    positions = next;
                }
                accepted.sort_unstable();
                accepted.dedup();
                accepted
            }
        }
    }
}

fn spend(budget: &mut usize) -> bool {
    if *budget == 0 {
        false
    } else {
        *budget -= 1;
        true
    }
}

/// Match the existing validator's ECMA escape translation. In particular, its
/// regular expression path uses ASCII digit/word classes and explicit ECMA
/// whitespace; its lookaround path leaves escapes to fancy-regex.
fn translate(pattern: &str) -> String {
    use regex_syntax::ast::{ErrorKind, parse::Parser};
    if let Err(error) = Parser::new().parse(pattern)
        && matches!(
            error.kind(),
            ErrorKind::UnsupportedLookAround | ErrorKind::UnsupportedBackreference
        )
    {
        return pattern.to_owned();
    }
    let mut chars = pattern.chars();
    let mut output = String::new();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match chars.next() {
            Some('d') => output.push_str("[0-9]"),
            Some('D') => output.push_str("[^0-9]"),
            Some('w') => output.push_str("[A-Za-z0-9_]"),
            Some('W') => output.push_str("[^A-Za-z0-9_]"),
            Some('s') => {
                output.push_str(r"[ \t\n\r\u{000b}\u{000c}\u{00a0}\u{feff}\u{2003}\u{2029}]")
            }
            Some('S') => {
                output.push_str(r"[^ \t\n\r\u{000b}\u{000c}\u{00a0}\u{feff}\u{2003}\u{2029}]")
            }
            Some('c') => match chars.next() {
                Some(letter) if letter.is_ascii_alphabetic() => {
                    output.push(char::from((letter as u8) % 32))
                }
                Some(other) => {
                    output.push_str(r"\c");
                    output.push(other);
                }
                None => output.push_str(r"\c"),
            },
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}
