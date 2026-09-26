// File: src/interpreter/environment.rs
//
// Lexical scoping environment for variable management in the Kujo interpreter.
// Implements a stack of scopes where inner scopes shadow outer scopes.

use super::value::Value;
use std::collections::{HashMap, HashSet};

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
    global_owner: std::sync::Arc<()>,
    capture_writes: Option<(usize, HashSet<(usize, String)>)>,
}

impl Environment {
    /// Track writes only for an async captured invocation. Ordinary environments
    /// allocate no journal. Same-binding read-modify-write remains non-atomic.
    pub(crate) fn track_capture_writes(&mut self) {
        self.capture_writes = Some((self.scopes.len(), HashSet::new()));
    }

    pub(crate) fn merge_capture_writes(&mut self, after: &Self) {
        let Some((_, writes)) = &after.capture_writes else {
            return;
        };
        for (index, name) in writes {
            if let (Some(target), Some(value)) = (
                self.scopes.get_mut(*index),
                after.scopes.get(*index).and_then(|scope| scope.get(name)),
            ) {
                target.insert(name.clone(), value.clone());
            }
        }
    }

    fn record_capture_write(
        journal: &mut Option<(usize, HashSet<(usize, String)>)>,
        index: usize,
        name: &str,
    ) {
        if let Some((limit, writes)) = journal {
            if index < *limit {
                writes.insert((index, name.to_owned()));
            }
        }
    }

    /// Create a new environment with a single global scope
    pub fn new() -> Self {
        Environment {
            scopes: vec![HashMap::new()],
            binding_kinds: vec![HashMap::new()],
            global_owner: std::sync::Arc::new(()),
            capture_writes: None,
        }
    }

    /// Snapshot lexical free bindings, retaining the original scope/kind layout.
    /// Top-level function values also need their global dependencies because
    /// they execute against this snapshot when called by the closure.
    pub(crate) fn capture_for_callable(
        &self,
        params: &[String],
        body: &[crate::ast::Stmt],
        lexical_self: Option<&str>,
    ) -> Self {
        let names = self
            .scopes
            .iter()
            .flat_map(|scope| scope.keys())
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|name| !params.contains(name) && lexical_self != Some(name.as_str()));
        let Ok(free) = crate::compiler::Compiler::spawn_free_bindings(body, names) else {
            // Preserve interpreter-only syntax when the bytecode resolver cannot
            // describe it; do not reject an otherwise valid AST callable.
            return self.clone();
        };
        let mut selected = vec![HashSet::new(); self.scopes.len()];
        let mut visited_containers = HashSet::new();
        let mut pending = Vec::new();
        for name in free {
            if let Some(index) = self.scopes.iter().rposition(|scope| scope.contains_key(&name)) {
                if selected[index].insert(name.clone()) {
                    pending.push(self.scopes[index][&name].clone());
                }
            }
        }
        while let Some(value) = pending.pop() {
            match value {
                Value::Function(params, body, None)
                | Value::AsyncFunction(params, body, None)
                | Value::GeneratorDef(params, body, None) => {
                    let names =
                        self.scopes[0].keys().filter(|name| !params.contains(name)).cloned();
                    let Ok(free) =
                        crate::compiler::Compiler::spawn_free_bindings(&body.get(), names)
                    else {
                        return self.clone();
                    };
                    for name in free {
                        if let Some(value) = self.scopes[0].get(&name) {
                            if selected[0].insert(name) {
                                pending.push(value.clone());
                            }
                        }
                    }
                }
                Value::Array(values) | Value::DenseIntDict(values) => {
                    if visited_containers.insert((0, std::sync::Arc::as_ptr(&values) as usize)) {
                        pending.extend(values.iter().cloned());
                    }
                }
                Value::FixedDict { values, .. } | Value::Set(values) | Value::Stack(values) => {
                    pending.extend(values)
                }
                Value::Queue(values) => pending.extend(values),
                Value::IntDict(values) => {
                    if visited_containers.insert((1, std::sync::Arc::as_ptr(&values) as usize)) {
                        pending.extend(values.values().cloned());
                    }
                }
                Value::StructDef { methods, .. } => pending.extend(methods.into_values()),
                Value::Dict(values) => {
                    if visited_containers.insert((2, std::sync::Arc::as_ptr(&values) as usize)) {
                        pending.extend(values.values().cloned());
                    }
                }
                Value::Struct { fields, .. } | Value::Tagged { fields, .. } => {
                    pending.extend(fields.into_values())
                }
                Value::Result { value, .. } | Value::Option { value, .. } => pending.push(*value),
                Value::Iterator { source, transformer, filter_fn, .. } => {
                    pending.push(*source);
                    pending.extend(transformer.map(|value| *value));
                    pending.extend(filter_fn.map(|value| *value));
                }
                _ => {}
            }
        }
        Self {
            scopes: self
                .scopes
                .iter()
                .enumerate()
                .map(|(index, scope)| {
                    scope
                        .iter()
                        .filter(|(name, _)| selected[index].contains(*name))
                        .map(|(name, value)| (name.clone(), value.clone()))
                        .collect()
                })
                .collect(),
            binding_kinds: self
                .binding_kinds
                .iter()
                .enumerate()
                .map(|(index, scope)| {
                    scope
                        .iter()
                        .filter(|(name, _)| selected[index].contains(*name))
                        .map(|(name, kind)| (name.clone(), *kind))
                        .collect()
                })
                .collect(),
            global_owner: self.global_owner.clone(),
            capture_writes: None,
        }
    }

    pub(crate) fn visible_bindings(&self) -> HashMap<String, (Value, BindingKind)> {
        let mut visible = HashMap::new();
        for (index, scope) in self.scopes.iter().enumerate() {
            for (name, value) in scope {
                visible.insert(
                    name.clone(),
                    (
                        value.clone(),
                        self.binding_kinds[index]
                            .get(name)
                            .copied()
                            .unwrap_or(BindingKind::Mutable),
                    ),
                );
            }
        }
        visible
    }

    pub(crate) fn global_snapshot(&self) -> Self {
        Self {
            scopes: vec![self.scopes[0].clone()],
            binding_kinds: vec![self.binding_kinds[0].clone()],
            global_owner: std::sync::Arc::new(()),
            capture_writes: None,
        }
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

    /// Enter a top-level function's lexical environment without cloning globals.
    /// Caller-local scopes are suspended, so assignment cannot mutate unrelated
    /// caller bindings. Captured closures use their own environment instead.
    pub(crate) fn enter_global_function(&mut self) -> Environment {
        let caller = Environment {
            scopes: self.scopes.split_off(1),
            binding_kinds: self.binding_kinds.split_off(1),
            global_owner: self.global_owner.clone(),
            capture_writes: None,
        };
        self.push_scope();
        caller
    }

    pub(crate) fn leave_global_function(&mut self, caller: Environment) {
        self.scopes.truncate(1);
        self.binding_kinds.truncate(1);
        self.scopes.extend(caller.scopes);
        self.binding_kinds.extend(caller.binding_kinds);
    }

    /// Swap only non-global scopes when suspending an execution owner. Global
    /// bindings remain live; caller scopes never leak into a resumed generator.
    pub(crate) fn swap_local_scopes(&mut self, mut owner: Environment) -> Environment {
        let previous = Environment {
            scopes: self.scopes.split_off(1),
            binding_kinds: self.binding_kinds.split_off(1),
            global_owner: self.global_owner.clone(),
            capture_writes: None,
        };
        self.scopes.append(&mut owner.scopes);
        self.binding_kinds.append(&mut owner.binding_kinds);
        previous
    }

    pub(crate) fn empty_scopes() -> Environment {
        Environment {
            scopes: Vec::new(),
            binding_kinds: Vec::new(),
            global_owner: std::sync::Arc::new(()),
            capture_writes: None,
        }
    }

    pub(crate) fn global_owner(&self) -> std::sync::Arc<()> {
        self.global_owner.clone()
    }

    /// Snapshot lexical bindings, without retaining the root environment.
    pub(crate) fn generator_environment(&self, capture_locals: bool) -> Self {
        let mut result = Self {
            scopes: vec![HashMap::new()],
            binding_kinds: vec![HashMap::new()],
            global_owner: self.global_owner.clone(),
            capture_writes: None,
        };
        if capture_locals {
            result.scopes.extend(self.scopes.iter().skip(1).cloned());
            result.binding_kinds.extend(self.binding_kinds.iter().skip(1).cloned());
        }
        result
    }

    /// Move the live globals into/out of a continuation without cloning them.
    pub(crate) fn swap_global_scope(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.scopes[0], &mut other.scopes[0]);
        std::mem::swap(&mut self.binding_kinds[0], &mut other.binding_kinds[0]);
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
        Self::record_capture_write(
            &mut self.capture_writes,
            self.scopes.len().saturating_sub(1),
            &name,
        );
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
        for (index, scope) in self.scopes.iter_mut().enumerate().rev() {
            if scope.contains_key(&name) {
                Self::record_capture_write(&mut self.capture_writes, index, &name);
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
                Self::record_capture_write(&mut self.capture_writes, scope_index, &name);
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
        for (index, scope) in self.scopes.iter_mut().enumerate().rev() {
            if let Some(value) = scope.get_mut(name) {
                Self::record_capture_write(&mut self.capture_writes, index, name);
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

                Self::record_capture_write(&mut self.capture_writes, scope_index, name);
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
