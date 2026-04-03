//! Code generation from JSON Schema documents.
//!
//! `jsoncompat_codegen` treats JSON Schema as the codegen boundary directly.
//! Generators accept one schema document at a time and optionally use
//! `x-jsoncompat` metadata emitted by `jsoncompat stamp` to preserve declaration
//! names and writer/reader wrapper roles.

mod dataclasses;

pub use dataclasses::{DataclassError, generate_dataclass_models};
use serde::{Deserialize, Serialize};

pub const JSONCOMPAT_METADATA_KEY: &str = "x-jsoncompat";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JsoncompatMetadata {
    Declaration {
        stable_id: String,
        name: String,
        version: u32,
        schema_ref: String,
    },
    Writer {
        stable_id: String,
        name: String,
        version: u32,
        payload_ref: String,
    },
    Reader {
        stable_id: String,
        name: String,
    },
    ReaderVariant {
        stable_id: String,
        name: String,
        version: u32,
        payload_ref: String,
    },
}
