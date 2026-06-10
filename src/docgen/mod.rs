pub mod adapters;
pub mod core;
pub mod discovery;
pub mod gaps;
pub mod model;
pub mod render;

#[derive(Debug, Clone)]
pub struct DocgenError {
    message: String,
}

impl DocgenError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for DocgenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DocgenError {}
