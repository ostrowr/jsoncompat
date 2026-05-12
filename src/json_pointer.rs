//! Small JSON Pointer builder shared by diagnostics and OpenAPI lowering.

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
        pointer.push(token);
        pointer
    }

    pub(crate) fn push(&mut self, token: impl Into<String>) {
        self.tokens.push(token.into());
    }

    pub(crate) fn prepend<'a>(&mut self, tokens: impl IntoIterator<Item = &'a str>) {
        let mut prefix = tokens
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        prefix.append(&mut self.tokens);
        self.tokens = prefix;
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
