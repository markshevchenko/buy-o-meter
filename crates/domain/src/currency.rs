#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Currency(String);

impl Currency {
    #[must_use]
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
