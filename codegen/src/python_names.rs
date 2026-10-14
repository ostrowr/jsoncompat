//! Readable, portable module names for generated Python identifiers.
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Name a generated field without shadowing Python keywords, runtime methods,
/// or annotation imports. Callers must also disambiguate names within a class.
pub fn field_name(json_name: &str) -> String {
    super::dataclasses::python_field_name(json_name)
}

/// Assign snake_case filenames. Only collisions and long names receive a digest;
/// adding an unrelated name never changes an existing assignment.
pub fn module_names<'a>(names: impl IntoIterator<Item = &'a str>) -> BTreeMap<String, String> {
    let mut by_stem: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for name in names {
        let mut stem = snake_case(name);
        if stem.is_empty() || stem.starts_with(|c: char| c.is_ascii_digit()) || reserved(&stem) {
            stem.insert_str(0, "model_");
        }
        by_stem.entry(stem).or_default().insert(name);
    }
    let mut result = BTreeMap::new();
    for (stem, names) in by_stem {
        let suffix = names.len() > 1 || stem.len() > 64;
        for name in names {
            let module = if suffix {
                let hash = format!("{:x}", Sha256::digest(name.as_bytes()));
                // snake_case never produces consecutive underscores. Reserve
                // this separator so a natural name cannot shadow a hashed one.
                format!("{}__{}", &stem[..stem.len().min(64)], &hash[..16])
            } else {
                stem.clone()
            };
            result.insert(name.into(), module);
        }
    }
    result
}
fn snake_case(name: &str) -> String {
    let chars: Vec<_> = name.chars().collect();
    let mut out = String::new();
    for (i, ch) in chars.iter().copied().enumerate() {
        if !ch.is_ascii_alphanumeric() {
            if !out.ends_with('_') {
                out.push('_');
            }
            continue;
        }
        if ch.is_ascii_uppercase()
            && i > 0
            && !out.ends_with('_')
            && (chars[i - 1].is_ascii_lowercase()
                || chars[i - 1].is_ascii_digit()
                || (chars[i - 1].is_ascii_uppercase()
                    && chars.get(i + 1).is_some_and(char::is_ascii_lowercase)))
        {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out.trim_matches('_').into()
}
fn reserved(name: &str) -> bool {
    matches!(
        name,
        "false"
            | "none"
            | "true"
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
            | "con"
            | "prn"
            | "aux"
            | "nul"
    ) || (name.len() == 4
        && (name.starts_with("com") || name.starts_with("lpt"))
        && matches!(name.as_bytes()[3], b'1'..=b'9'))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_are_readable_stable_and_portable() {
        let long = "LongName".repeat(40);
        let names = module_names([
            "GetPet",
            "HTTPResponse",
            "URL",
            "Url",
            "Class",
            "CON",
            &long,
        ]);
        assert_eq!(names["GetPet"], "get_pet");
        assert_eq!(names["HTTPResponse"], "http_response");
        assert_eq!(names["Class"], "model_class");
        assert_eq!(names["CON"], "model_con");
        assert_ne!(names["URL"], names["Url"]);
        assert!(names.values().all(|s| s.len() <= 82));
        assert_eq!(module_names(["GetPet", "Added"])["GetPet"], names["GetPet"]);
        assert_eq!(module_names(["GetPet", "GetPet"])["GetPet"], "get_pet");
        let shadow = names["URL"].clone();
        let collision = module_names(["URL", "Url", &shadow]);
        assert_eq!(collision.values().collect::<BTreeSet<_>>().len(), 3);
    }
}
