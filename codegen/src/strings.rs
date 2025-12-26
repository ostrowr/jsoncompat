use crate::error::CodegenError;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct NameAllocator {
    used: HashSet<String>,
}

impl NameAllocator {
    pub fn allocate(&mut self, base: &str) -> Result<String, CodegenError> {
        let candidate = if base.is_empty() {
            "Model".to_string()
        } else {
            base.to_string()
        };

        if !self.used.contains(&candidate) {
            self.used.insert(candidate.clone());
            return Ok(candidate);
        }

        for idx in 2.. {
            let next = format!("{candidate}{idx}");
            if !self.used.contains(&next) {
                self.used.insert(next.clone());
                return Ok(next);
            }
        }

        Err(CodegenError::NameConflict { name: candidate })
    }
}

pub fn sanitize_type_name(input: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize {
                out.push(ch.to_ascii_uppercase());
            } else {
                out.push(ch.to_ascii_lowercase());
            }
            capitalize = false;
        } else {
            capitalize = true;
        }
    }

    if out.is_empty() {
        return "Model".to_string();
    }

    if out
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        out = format!("Model{out}");
    }

    if is_python_keyword(&out) {
        out.push_str("Model");
    }

    out
}

pub fn sanitize_field_name(input: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;
    let mut prev_was_lower = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            let is_upper = ch.is_ascii_uppercase();
            if is_upper && prev_was_lower {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_underscore = false;
            prev_was_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            if !prev_underscore {
                out.push('_');
                prev_underscore = true;
            }
            prev_was_lower = false;
        }
    }

    let trimmed = out.trim_matches('_').to_string();
    let mut result = if trimmed.is_empty() {
        "field".to_string()
    } else {
        trimmed
    };

    if result
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        result = format!("field_{result}");
    }

    if is_python_keyword(&result) {
        result = format!("field_{result}");
    }

    result
}

fn is_python_keyword(value: &str) -> bool {
    matches!(
        value,
        "False"
            | "None"
            | "True"
            | "and"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
    )
}
