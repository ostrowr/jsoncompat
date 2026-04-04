//! Back‑compatibility checking library.
//!
//! This crate depends on `json_schema_ast`, which provides a strict
//! in‑memory representation (`SchemaDocument` / `SchemaNode`) of a
//! Draft 2020‑12 JSON Schema.  The only responsibility of this crate is to
//! offer algorithms that compare two schemas and decide whether a change is
//! backward‑compatible from the point of view of a serializer or deserializer.

// Re‑export the fundamental building blocks from the core schema crate so that
// downstream crates can just depend on *this* crate for both parsing and
// compatibility checking if they wish.
pub use json_schema_ast::{
    ContainsConstraint, CountRange, IntegerBounds, NodeId, NumberBound, NumberBounds,
    PatternConstraint, PatternProperty, PatternSupport, SchemaBuildError, SchemaDocument,
    SchemaNode, SchemaNodeKind,
};

mod subset;

pub use subset::{is_subschema_of, type_constraints_subsumed};

/// The role under which a compatibility check is performed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    /// Evolving the *producer* (serializer).  A change is safe if every value
    /// produced by the _new_ schema is still accepted by the _old_ one.
    Serializer,
    /// Evolving the *consumer* (deserializer).  A change is safe if every value
    /// accepted by the _old_ schema is still valid under the _new_ one.
    Deserializer,
    /// We need to maintain full equivalence in both directions.
    Both,
}

/// Top‑level convenience wrapper:
///
/// * `Role::Serializer`   ⇒   `new ⊆ old`
/// * `Role::Deserializer` ⇒   `old ⊆ new`
/// * `Role::Both`         ⇒   bidirectional inclusion
pub fn check_compat(
    old: &SchemaDocument,
    new: &SchemaDocument,
    role: Role,
) -> Result<bool, SchemaBuildError> {
    let old = old.root()?;
    let new = new.root()?;

    match role {
        Role::Serializer => Ok(is_subschema_of(new, old)),
        Role::Deserializer => Ok(is_subschema_of(old, new)),
        Role::Both => Ok(is_subschema_of(new, old) && is_subschema_of(old, new)),
    }
}
