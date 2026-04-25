/// A single generics scope
pub type Generics = Vec<String>;

/// Defines stack of generics scopes (ribs).
#[derive(Default)]
pub struct GenericsStack {
    stack: Vec<Generics>,
}

/// Implementation
impl GenericsStack {
    /// Enters a generic scope.
    pub fn enter(&mut self, generics: Vec<String>) {
        self.stack.push(generics)
    }

    /// Exits current generics scope.
    pub fn exit(&mut self) {
        self.stack.pop();
    }

    /// Looks up a generic by name, searching from innermost to outermost scope.
    pub fn lookup(&self, name: &str) -> Option<usize> {
        self.stack
            .last()
            .and_then(|s| s.iter().position(|p| p == name))
    }
}
