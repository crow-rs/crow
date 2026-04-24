/// Imports
use crate::typ::Typ;
use crow_macros::bug;
use std::collections::HashMap;

/// A single lexical scope that mappings
/// variable names to their types.
pub type Rib = HashMap<String, Typ>;

/// Defines stack of lexical scopes (ribs).
#[derive(Default)]
pub struct RibsStack {
    stack: Vec<Rib>,
}

/// Implementation
impl RibsStack {
    /// Pushes a new, empty rib onto the stack.
    pub fn push(&mut self) {
        self.stack.push(HashMap::new())
    }

    /// Pops the top rib out the stack.
    pub fn pop(&mut self) -> Option<Rib> {
        self.stack.pop()
    }

    /// Defines a variable in the current scope.
    pub fn define(&mut self, name: &str, typ: Typ) {
        match self.stack.last_mut() {
            Some(env) => {
                env.insert(name.to_owned(), typ);
            }
            None => bug!("no scopes in ribs stack"),
        }
    }

    /// Looks up a variable by name, searching from innermost to outermost scope.
    pub fn lookup(&self, name: &str) -> Option<Typ> {
        for env in self.stack.iter().rev() {
            if env.contains_key(name) {
                return Some(env.get(name).unwrap().clone());
            }
        }
        None
    }
}
