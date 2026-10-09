//! Small, deterministic proof regressions, including empty languages that
//! cannot be exercised by the generated-value roundtrip fixtures.
use jsoncompat::{
    Role, SchemaDocument, check_compat, compatibility_warnings, explain_compat_failure,
};
use serde::Deserialize;
use serde_json::Value;
use std::{fs, path::Path};

datatest_stable::harness! {
    { test = fixture, root = "tests/fixtures/compatibility", pattern = r".*\.json$" },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    description: String,
    old: Value,
    new: Value,
    // Forward serializer/deserializer, then reverse serializer/deserializer.
    expected: [bool; 4],
    witnesses: Vec<Witness>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness {
    data: Value,
    old: bool,
    new: bool,
}

fn fixture(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cases: Vec<Case> = serde_json::from_slice(&fs::read(path)?)?;
    for case in cases {
        let old = SchemaDocument::from_json(&case.old)?;
        let new = SchemaDocument::from_json(&case.new)?;
        let old_canonical = json_schema_ast::compile(old.canonical_schema_json()?)?;
        let new_canonical = json_schema_ast::compile(new.canonical_schema_json()?)?;
        for schema in [&old, &new] {
            assert!(
                compatibility_warnings(schema)?.is_empty(),
                "{} must model every assertion",
                case.description
            );
        }
        assert!(
            !case.witnesses.is_empty(),
            "{} needs independent witnesses",
            case.description
        );
        for witness in &case.witnesses {
            for (schema, canonical, expected) in [
                (&old, &old_canonical, witness.old),
                (&new, &new_canonical, witness.new),
            ] {
                assert_eq!(
                    canonical.is_valid(&witness.data),
                    expected,
                    "{}: canonical witness {}",
                    case.description,
                    witness.data
                );
                assert_eq!(
                    schema.root()?.accepts_value(&witness.data),
                    expected,
                    "{}: resolved witness {}",
                    case.description,
                    witness.data
                );
            }
            assert_eq!(
                old.is_valid(&witness.data)?,
                witness.old,
                "{}: old witness {}",
                case.description,
                witness.data
            );
            assert_eq!(
                new.is_valid(&witness.data)?,
                witness.new,
                "{}: new witness {}",
                case.description,
                witness.data
            );
            if case.expected[0] {
                assert!(
                    !witness.new || witness.old,
                    "{}: serializer label contradicts witness {}",
                    case.description,
                    witness.data
                );
            }
            if case.expected[2] {
                assert!(
                    !witness.old || witness.new,
                    "{}: reverse serializer label contradicts witness {}",
                    case.description,
                    witness.data
                );
            }
        }
        for (before, after, expected) in [
            (&old, &new, &case.expected[..2]),
            (&new, &old, &case.expected[2..]),
        ] {
            for (role, expected) in [
                (Role::Serializer, expected[0]),
                (Role::Deserializer, expected[1]),
                (Role::Both, expected[0] && expected[1]),
            ] {
                let explanation = explain_compat_failure(before, after, role)?;
                assert_eq!(
                    check_compat(before, after, role)?,
                    expected,
                    "{}: {role:?}: {explanation:?}\nold: {}\nnew: {}",
                    case.description,
                    before.source_schema_json(),
                    after.source_schema_json()
                );
                assert_eq!(
                    explanation.is_none(),
                    expected,
                    "{}: {role:?} explanation disagrees with verdict",
                    case.description
                );
            }
        }
    }
    Ok(())
}
