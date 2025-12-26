use serde_json::Value;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemaPath {
    segments: Vec<String>,
}

impl SchemaPath {
    pub fn root() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn push<S: Into<String>>(&self, segment: S) -> Self {
        let mut next = self.segments.clone();
        next.push(segment.into());
        Self { segments: next }
    }

    pub fn as_segments(&self) -> &[String] {
        &self.segments
    }
}

impl fmt::Display for SchemaPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            return write!(f, "#");
        }
        write!(f, "#")?;
        for segment in &self.segments {
            write!(f, "/{}", escape_json_pointer(segment))?;
        }
        Ok(())
    }
}

fn escape_json_pointer(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("invalid schema at {location}: {message}")]
    InvalidSchema {
        location: SchemaPath,
        message: String,
    },

    #[error("unsupported schema feature at {location}: {feature}")]
    UnsupportedFeature {
        location: SchemaPath,
        feature: String,
    },

    #[error("unsupported enum/const value at {location}: {value}")]
    UnsupportedEnumValue { location: SchemaPath, value: Value },

    #[error("invalid $ref at {location}: {ref_path} ({message})")]
    InvalidRef {
        location: SchemaPath,
        ref_path: String,
        message: String,
    },

    #[error("unknown $ref at {location}: {ref_path}")]
    RefNotFound {
        location: SchemaPath,
        ref_path: String,
    },

    #[error("root schema must be an object schema (found {found})")]
    RootNotObject { found: String },

    #[error("default value at {location} does not match the schema: {message}")]
    InvalidDefault {
        location: SchemaPath,
        message: String,
    },

    #[error("model name conflict: {name}")]
    NameConflict { name: String },
}
