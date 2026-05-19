//! Small JSON Pointer builder for OpenAPI validation diagnostics.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JsonPointer {
    tokens: Vec<String>,
}

impl JsonPointer {
    pub(crate) fn root() -> Self {
        Self { tokens: Vec::new() }
    }

    pub(crate) fn child(&self, token: impl Into<String>) -> Self {
        let mut pointer = self.clone();
        pointer.tokens.push(token.into());
        pointer
    }

    pub(crate) fn tokens(&self) -> impl Iterator<Item = &str> {
        self.tokens.iter().map(String::as_str)
    }

    pub(crate) fn render(&self) -> String {
        if self.tokens.is_empty() {
            return "#".to_owned();
        }
        let suffix = self
            .tokens
            .iter()
            .map(|token| token.replace('~', "~0").replace('/', "~1"))
            .collect::<Vec<_>>()
            .join("/");
        format!("#/{suffix}")
    }
}
