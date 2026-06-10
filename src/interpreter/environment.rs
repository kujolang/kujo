// File: src/interpreter/environment.rs
//
// Lexical scoping environment for variable management in the Kujo interpreter.
// Implements a stack of scopes where inner scopes shadow outer scopes.

use super::value::Value;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    Mutable,
    LetImmutable,
    Const,
}

impl BindingKind {
    fn reassignment_error(self, name: &str) -> String {
        match self {
            BindingKind::Mutable => unreachable!("mutable bindings allow reassignment"),
            BindingKind::LetImmutable => {
                format!("Cannot reassign immutable let binding: {}", name)
            }
            BindingKind::Const => format!("Cannot reassign const binding: {}", name),
        }
    }

    fn mutation_error(self, name: &str) -> String {
        match self {
            BindingKind::Mutable => unreachable!("mutable bindings allow mutation"),
            BindingKind::LetImmutable => format!("Cannot mutate immutable let binding: {}", name),
            BindingKind::Const => format!("Cannot mutate const binding: {}", name),
        }
    }

    fn allows_mutation(self) -> bool {
        matches!(self, BindingKind::Mutable)
    }
}

/// Variable storage using lexical scoping
///
/// The Environment maintains a stack of scopes (Vec<HashMap>). When looking up
/// a variable, we search from the innermost scope (end of Vec) outward. This
/// implements proper lexical scoping with shadowing.
///
/// # Examples
///
/// ```ignore
/// use kujo::interpreter::{Environment, Value};
///
/// let mut env = Environment::new();
/// env.define("x".to_string(), Value::Int(10));  // Global scope
///
/// env.push_scope();                             // Enter function scope
/// env.define("x".to_string(), Value::Int(20));  // Shadows outer x
/// assert!(matches!(env.get("x"), Some(Value::Int(20))));
///
/// env.pop_scope();                              // Exit function scope
/// assert!(matches!(env.get("x"), Some(Value::Int(10))));  // Original x visible again
/// ```
#[derive(Clone, Debug)]
pub struct Environment {
    pub scopes: Vec<HashMap<String, Value>>,
    binding_kinds: Vec<HashMap<String, BindingKind>>,
}

impl Environment {
    /// Create a new environment with a single global scope
    pub fn new() -> Self {
        Environment { scopes: vec![HashMap::new()], binding_kinds: vec![HashMap::new()] }
    }

    /// Push a new scope onto the stack (e.g., entering a function)
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.binding_kinds.push(HashMap::new());
    }

    /// Pop the innermost scope from the stack (e.g., exiting a function)
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
            self.binding_kinds.pop();
        }
    }

    /// Get a variable from the environment, searching from inner to outer scopes
    /// Returns a cloned value if found
    pub fn get(&self, name: &str) -> Option<Value> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    /// Define a new variable in the current (innermost) scope
    pub fn define(&mut self, name: String, value: Value) {
        self.define_with_kind(name, value, BindingKind::Mutable);
    }

    pub fn define_with_kind(&mut self, name: String, value: Value, kind: BindingKind) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.clone(), value);
        }
        if let Some(kinds) = self.binding_kinds.last_mut() {
            kinds.insert(name, kind);
        }
    }

    pub fn define_with_kind_checked(
        &mut self,
        name: String,
        value: Value,
        kind: BindingKind,
    ) -> Result<(), String> {
        if self.current_scope_contains(name.as_str()) {
            return Err(format!("Duplicate declaration in the same scope: {}", name));
        }

        self.define_with_kind(name, value, kind);
        Ok(())
    }

    fn current_scope_contains(&self, name: &str) -> bool {
        self.scopes.last().map(|scope| scope.contains_key(name)).unwrap_or(false)
    }

    /// Set an existing variable, searching from inner to outer scopes
    /// If not found, creates it in the current scope
    pub fn set(&mut self, name: String, value: Value) {
        // Try to find and update existing variable
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(&name) {
                scope.insert(name, value);
                return;
            }
        }
        // If not found, create in current scope
        self.define(name, value);
    }

    pub fn assign_checked(&mut self, name: String, value: Value) -> Result<(), String> {
        for (scope_index, scope) in self.scopes.iter_mut().enumerate().rev() {
            if scope.contains_key(&name) {
                let kind = self.binding_kinds[scope_index]
                    .get(&name)
                    .copied()
                    .unwrap_or(BindingKind::Mutable);
                if !kind.allows_mutation() {
                    return Err(kind.reassignment_error(&name));
                }
                scope.insert(name, value);
                return Ok(());
            }
        }

        // Preserve existing Kujo behavior: assignment can create a new mutable binding.
        self.define(name, value);
        Ok(())
    }

    /// Mutate an existing variable using a closure
    ///
    /// This is useful for in-place modifications like `x += 1` where we want to
    /// read the current value, modify it, and write it back atomically.
    #[allow(dead_code)]
    pub fn mutate<F>(&mut self, name: &str, f: F) -> bool
    where
        F: FnOnce(&mut Value),
    {
        // Find the scope containing this variable
        for scope in self.scopes.iter_mut().rev() {
            if let Some(value) = scope.get_mut(name) {
                f(value);
                return true;
            }
        }
        false
    }

    pub fn mutate_checked<F>(&mut self, name: &str, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut Value),
    {
        for (scope_index, scope) in self.scopes.iter_mut().enumerate().rev() {
            if let Some(value) = scope.get_mut(name) {
                let kind = self.binding_kinds[scope_index]
                    .get(name)
                    .copied()
                    .unwrap_or(BindingKind::Mutable);
                if !kind.allows_mutation() {
                    return Err(kind.mutation_error(name));
                }

                f(value);
                return Ok(());
            }
        }

        Err(format!("Undefined variable: {}", name))
    }

    pub fn ensure_mutable_for_mutation(&self, name: &str) -> Result<(), String> {
        for (scope_index, scope) in self.scopes.iter().enumerate().rev() {
            if scope.contains_key(name) {
                let kind = self.binding_kinds[scope_index]
                    .get(name)
                    .copied()
                    .unwrap_or(BindingKind::Mutable);
                if kind.allows_mutation() {
                    return Ok(());
                }
                return Err(kind.mutation_error(name));
            }
        }

        Err(format!("Undefined variable: {}", name))
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
