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
    /// Explicit writable bindings. Module-native defaults may be stored separately;
    /// use `get` for lexical lookup, including those unchanged preloads.
    pub scopes: Vec<HashMap<String, Value>>,
    // Module snapshots share only immutable native name metadata. Mutable globals
    // remain ordinary owned maps, including every override of a default.
    native_defaults: Option<std::sync::Arc<HashMap<String, String>>>,
    // Missing entries mean Mutable. Store only immutable exceptions, avoiding
    // duplicate name storage for native preloads and ordinary mutable bindings.
    binding_kinds: Vec<HashMap<String, BindingKind>>,
    global_owner: std::sync::Arc<()>,
    capture_writes: Option<(usize, HashSet<(usize, String)>)>,
    // Allocation-free conservative proof for read-only synchronous activations.
    // Local parameters are outside the captured prefix; unknown transitions
    // invalidate the proof rather than assuming they cannot change captures.
    capture_observation: Option<(usize, bool)>,
}

impl Environment {
    pub(crate) fn observe_capture_writes(&mut self, inherit: bool) {
        let changed = inherit && self.capture_observation.map_or(true, |(_, changed)| changed);
        self.capture_observation = Some((self.scopes.len(), changed));
    }

    pub(crate) fn capture_is_unchanged(&self) -> bool {
        matches!(self.capture_observation, Some((_, false)))
    }

    pub(crate) fn invalidate_capture_observation(&mut self) {
        if let Some((_, changed)) = &mut self.capture_observation {
            *changed = true;
        }
    }

    /// Preserve capture scope/kind layout without copying recursive parameters.
    pub(crate) fn capture_prefix_snapshot(&self, count: usize) -> Self {
        Self {
            scopes: self.scopes[..count].to_vec(),
            binding_kinds: self.binding_kinds[..count].to_vec(),
            native_defaults: self.native_defaults.clone(),
            global_owner: self.global_owner.clone(),
            capture_writes: self.capture_writes.clone(),
            capture_observation: self.capture_observation,
        }
    }

    pub(crate) fn copy_global_scope_from(&mut self, source: &Self) {
        self.invalidate_capture_observation();
        self.scopes[0].clone_from(&source.scopes[0]);
        self.binding_kinds[0].clone_from(&source.binding_kinds[0]);
        self.native_defaults.clone_from(&source.native_defaults);
        if let (Some((_, target)), Some((_, writes))) =
            (&mut self.capture_writes, &source.capture_writes)
        {
            target.extend(writes.iter().filter(|(index, _)| *index == 0).cloned());
        }
    }

    pub(crate) fn restore_capture_locals(&mut self, completed: &Self, count: usize) {
        self.invalidate_capture_observation();
        self.scopes[1..count].clone_from_slice(&completed.scopes[1..count]);
        self.binding_kinds[1..count].clone_from_slice(&completed.binding_kinds[1..count]);
        if let (Some((_, target)), Some((_, writes))) =
            (&mut self.capture_writes, &completed.capture_writes)
        {
            target.extend(writes.iter().filter(|(index, _)| *index > 0 && *index < count).cloned());
        }
    }
    /// Compact only unchanged, mutable, canonical builtin preloads in a module
    /// capture. Aliases, user overrides and immutable declarations remain stored.
    /// No shared Value or mutable map can reach a native globals pointer here.
    pub(crate) fn share_module_native_defaults(&mut self) {
        if self.scopes.len() != 1 || self.native_defaults.is_some() {
            return;
        }
        let mut defaults = HashMap::new();
        self.scopes[0].retain(|name, value| {
            let eligible = matches!(value, Value::NativeFunction(native) if native == name)
                && super::Interpreter::is_builtin_name(name)
                && !self.binding_kinds[0].contains_key(name);
            if eligible {
                let Value::NativeFunction(native) = value else { unreachable!() };
                defaults.insert(name.clone(), native.clone());
            }
            !eligible
        });
        if !defaults.is_empty() {
            self.scopes[0].shrink_to_fit();
            self.native_defaults = Some(std::sync::Arc::new(defaults));
        }
    }

    fn native_default(&self, name: &str) -> Option<Value> {
        self.native_defaults.as_ref()?.get(name).cloned().map(Value::NativeFunction)
    }

    /// Test lexical membership without cloning a value. Shared module defaults
    /// are bindings too, including when native loops choose an assignment owner.
    pub(crate) fn contains_binding(&self, name: &str) -> bool {
        self.scopes.iter().any(|scope| scope.contains_key(name))
            || self.native_defaults.as_ref().is_some_and(|defaults| defaults.contains_key(name))
    }

    fn set_native_override(&mut self, name: &str, value: Value) {
        Self::record_capture_write(
            &mut self.capture_writes,
            &mut self.capture_observation,
            0,
            name,
        );
        self.scopes[0].insert(name.to_owned(), value);
    }

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
        observation: &mut Option<(usize, bool)>,
        index: usize,
        name: &str,
    ) {
        if let Some((limit, changed)) = observation {
            if index < *limit {
                *changed = true;
            }
        }
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
            native_defaults: None,
            global_owner: std::sync::Arc::new(()),
            capture_writes: None,
            capture_observation: None,
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
            .chain(self.native_defaults.iter().flat_map(|map| map.keys()))
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
        let mut selected_defaults = HashMap::new();
        let mut visited_containers = HashSet::new();
        let mut pending = Vec::new();
        for name in free {
            if let Some(index) = self.scopes.iter().rposition(|scope| scope.contains_key(&name)) {
                if selected[index].insert(name.clone()) {
                    pending.push(self.scopes[index][&name].clone());
                }
            } else if let Some(native) =
                self.native_defaults.as_ref().and_then(|map| map.get(&name))
            {
                selected_defaults.insert(name, native.clone());
            }
        }
        while let Some(value) = pending.pop() {
            match value {
                Value::Function(params, body, None)
                | Value::AsyncFunction(params, body, None)
                | Value::GeneratorDef(params, body, None) => {
                    let names = self.scopes[0]
                        .keys()
                        .chain(self.native_defaults.iter().flat_map(|map| map.keys()))
                        .filter(|name| !params.contains(name))
                        .cloned();
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
                        } else if let Some(native) =
                            self.native_defaults.as_ref().and_then(|map| map.get(&name))
                        {
                            selected_defaults.insert(name, native.clone());
                        }
                    }
                }
                Value::Array(values)
                | Value::DenseIntDict(values)
                | Value::FixedDict { values, .. } => {
                    if visited_containers.insert((0, std::sync::Arc::as_ptr(&values) as usize)) {
                        pending.extend(values.iter().cloned());
                    }
                }
                Value::Set(values) => pending.extend(values),
                Value::Queue(values) => pending.extend(values.iter().cloned()),
                Value::Stack(values) => pending.extend(values.iter().cloned()),
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
                Value::Struct { fields, .. } => {
                    if visited_containers.insert((3, std::sync::Arc::as_ptr(&fields) as usize)) {
                        pending.extend(fields.values().cloned());
                    }
                }
                Value::Tagged { fields, .. } => pending.extend(fields.into_values()),
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
            native_defaults: (!selected_defaults.is_empty())
                .then(|| std::sync::Arc::new(selected_defaults)),
            global_owner: self.global_owner.clone(),
            capture_writes: None,
            capture_observation: None,
        }
    }

    pub(crate) fn release_rebound_local(
        &mut self,
        name: &str,
        first_scope: usize,
        argument: &Value,
    ) {
        let Some(index) = self.scopes.iter().rposition(|scope| scope.contains_key(name)) else {
            return;
        };
        if index >= first_scope
            && self.binding_kinds[index].get(name).copied().unwrap_or(BindingKind::Mutable)
                == BindingKind::Mutable
        {
            let value = self.scopes[index].get_mut(name).unwrap();
            if super::assignment::same_storage(value, argument) {
                *value = Value::Null;
            }
        }
    }

    pub(crate) fn visible_bindings(&self) -> HashMap<String, (Value, BindingKind)> {
        let mut visible = self
            .native_defaults
            .iter()
            .flat_map(|map| map.iter())
            .map(|(name, native)| {
                (name.clone(), (Value::NativeFunction(native.clone()), BindingKind::Mutable))
            })
            .collect::<HashMap<_, _>>();
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
            native_defaults: self.native_defaults.clone(),
            global_owner: std::sync::Arc::new(()),
            capture_writes: None,
            capture_observation: None,
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
            native_defaults: self.native_defaults.clone(),
            global_owner: self.global_owner.clone(),
            capture_writes: None,
            capture_observation: None,
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
            native_defaults: self.native_defaults.clone(),
            global_owner: self.global_owner.clone(),
            capture_writes: None,
            capture_observation: None,
        };
        self.scopes.append(&mut owner.scopes);
        self.binding_kinds.append(&mut owner.binding_kinds);
        previous
    }

    pub(crate) fn empty_scopes() -> Environment {
        Environment {
            scopes: Vec::new(),
            binding_kinds: Vec::new(),
            native_defaults: None,
            global_owner: std::sync::Arc::new(()),
            capture_writes: None,
            capture_observation: None,
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
            native_defaults: self.native_defaults.clone(),
            global_owner: self.global_owner.clone(),
            capture_writes: None,
            capture_observation: None,
        };
        if capture_locals {
            result.scopes.extend(self.scopes.iter().skip(1).cloned());
            result.binding_kinds.extend(self.binding_kinds.iter().skip(1).cloned());
        }
        result
    }

    /// Move the live globals into/out of a continuation without cloning them.
    pub(crate) fn swap_global_scope(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.native_defaults, &mut other.native_defaults);
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
        self.native_default(name)
    }

    /// Define a new variable in the current (innermost) scope
    pub fn define(&mut self, name: String, value: Value) {
        self.define_with_kind(name, value, BindingKind::Mutable);
    }

    pub fn define_with_kind(&mut self, name: String, value: Value, kind: BindingKind) {
        Self::record_capture_write(
            &mut self.capture_writes,
            &mut self.capture_observation,
            self.scopes.len().saturating_sub(1),
            &name,
        );
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.clone(), value);
        }
        if let Some(kinds) = self.binding_kinds.last_mut() {
            if kind == BindingKind::Mutable {
                // A definition can replace an immutable binding through host APIs;
                // remove its old exception rather than leaving stale permissions.
                kinds.remove(&name);
            } else {
                kinds.insert(name, kind);
            }
        }
    }

    pub fn define_with_kind_checked(
        &mut self,
        name: String,
        value: Value,
        kind: BindingKind,
    ) -> Result<(), String> {
        if self.current_scope_contains(name.as_str()) {
            // Native functions are preloaded into the interpreter's global scope,
            // while the VM resolves them only when no lexical binding exists.
            // Let source declarations shadow that preload so both engines apply
            // the same lexical-name rules. A second source declaration still
            // sees the non-native replacement and remains an error.
            let shadows_preloaded_native = self
                .scopes
                .last()
                .and_then(|scope| scope.get(&name))
                .is_some_and(|existing| matches!(existing, Value::NativeFunction(_)));
            if !shadows_preloaded_native {
                return Err(format!("Duplicate declaration in the same scope: {}", name));
            }
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
                Self::record_capture_write(
                    &mut self.capture_writes,
                    &mut self.capture_observation,
                    index,
                    &name,
                );
                scope.insert(name, value);
                return;
            }
        }
        if self.native_defaults.as_ref().is_some_and(|map| map.contains_key(&name)) {
            self.set_native_override(&name, value);
            return;
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
                Self::record_capture_write(
                    &mut self.capture_writes,
                    &mut self.capture_observation,
                    scope_index,
                    &name,
                );
                scope.insert(name, value);
                return Ok(());
            }
        }

        if self.native_defaults.as_ref().is_some_and(|map| map.contains_key(&name)) {
            self.set_native_override(&name, value);
            return Ok(());
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
                Self::record_capture_write(
                    &mut self.capture_writes,
                    &mut self.capture_observation,
                    index,
                    name,
                );
                f(value);
                return true;
            }
        }
        if let Some(mut value) = self.native_default(name) {
            f(&mut value);
            self.set_native_override(name, value);
            return true;
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

                Self::record_capture_write(
                    &mut self.capture_writes,
                    &mut self.capture_observation,
                    scope_index,
                    name,
                );
                f(value);
                return Ok(());
            }
        }

        if let Some(mut value) = self.native_default(name) {
            f(&mut value);
            self.set_native_override(name, value);
            return Ok(());
        }
        Err(format!("Undefined variable: {}", name))
    }

    /// An atomic, fallible mutation reports whether it actually changed a binding.
    /// Callers must leave the value untouched on Err/false. In particular, a
    /// rejected optimization must not appear in a callback's captured-write log.
    pub(crate) fn try_mutate_checked(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Value) -> Result<bool, String>,
    ) -> Result<bool, String> {
        for (scope_index, scope) in self.scopes.iter_mut().enumerate().rev() {
            if let Some(value) = scope.get_mut(name) {
                let kind = self.binding_kinds[scope_index]
                    .get(name)
                    .copied()
                    .unwrap_or(BindingKind::Mutable);
                if !kind.allows_mutation() {
                    return Err(kind.mutation_error(name));
                }
                let changed = f(value)?;
                if changed {
                    Self::record_capture_write(
                        &mut self.capture_writes,
                        &mut self.capture_observation,
                        scope_index,
                        name,
                    );
                }
                return Ok(changed);
            }
        }
        if let Some(mut value) = self.native_default(name) {
            let changed = f(&mut value)?;
            if changed {
                self.set_native_override(name, value);
            }
            return Ok(changed);
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

        if self.native_defaults.as_ref().is_some_and(|map| map.contains_key(name)) {
            return Ok(());
        }
        Err(format!("Undefined variable: {}", name))
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod conditional_mutation_tests {
    use super::*;

    #[test]
    fn rejected_mutations_do_not_publish_capture_writes() {
        let mut env = Environment::new();
        env.define("items".into(), Value::Int(1));
        env.track_capture_writes();
        assert!(!env.try_mutate_checked("items", |_| Ok(false)).unwrap());
        assert!(env.try_mutate_checked("items", |_| Err("rejected".into())).is_err());
        assert!(env.capture_writes.as_ref().unwrap().1.is_empty());
        assert!(env
            .try_mutate_checked("items", |value| {
                *value = Value::Int(2);
                Ok(true)
            })
            .unwrap());
        assert_eq!(env.capture_writes.as_ref().unwrap().1.len(), 1);
    }
}

#[cfg(test)]
mod permission_snapshot_tests {
    use super::*;

    #[test]
    fn sparse_permissions_preserve_snapshot_immutability_and_redefinition() {
        let mut original = Environment::new();
        original.define_with_kind("locked".into(), Value::Int(1), BindingKind::Const);
        original.define("mutable".into(), Value::Int(7));
        assert_eq!(original.binding_kinds[0].len(), 1);
        let mut snapshot = original.clone();
        snapshot.define("locked".into(), Value::Int(2));
        assert!(snapshot.binding_kinds[0].is_empty());
        snapshot.assign_checked("locked".into(), Value::Int(3)).unwrap();
        assert!(original.assign_checked("locked".into(), Value::Int(4)).is_err());
        assert!(matches!(original.get("locked"), Some(Value::Int(1))));
        snapshot.push_scope();
        snapshot.define_with_kind("local".into(), Value::Int(5), BindingKind::LetImmutable);
        let child = snapshot.clone();
        snapshot.pop_scope();
        assert!(child.ensure_mutable_for_mutation("local").is_err());
        assert!(snapshot.get("local").is_none());
    }
}

#[cfg(test)]
mod module_native_default_tests {
    use super::*;

    fn compact() -> Environment {
        let mut env = super::super::Interpreter::new().env;
        env.share_module_native_defaults();
        env
    }

    #[test]
    fn native_defaults_are_shared_but_overrides_and_permissions_are_snapshots() {
        let mut original = compact();
        let mut snapshot = original.clone();
        assert!(std::sync::Arc::ptr_eq(
            original.native_defaults.as_ref().unwrap(),
            snapshot.native_defaults.as_ref().unwrap()
        ));
        assert!(original.scopes[0].len() < 32);
        assert!(matches!(original.get("abs"), Some(Value::NativeFunction(name)) if name == "abs"));
        snapshot.push_scope();
        snapshot.assign_checked("abs".into(), Value::Int(7)).unwrap();
        snapshot.pop_scope();
        assert!(matches!(snapshot.get("abs"), Some(Value::Int(7))));
        assert!(matches!(original.get("abs"), Some(Value::NativeFunction(_))));
        original.define_with_kind("abs".into(), Value::Int(9), BindingKind::Const);
        assert!(original.assign_checked("abs".into(), Value::Null).is_err());
        assert!(original.ensure_mutable_for_mutation("abs").is_err());
        assert!(matches!(snapshot.get("abs"), Some(Value::Int(7))));
        assert!(matches!(snapshot.visible_bindings()["abs"].0, Value::Int(7)));
        assert!(matches!(snapshot.visible_bindings()["sqrt"].0, Value::NativeFunction(_)));
    }

    #[test]
    fn default_override_writes_merge_and_rejected_mutations_do_not_publish() {
        let mut env = compact();
        let mut invocation = env.clone();
        invocation.track_capture_writes();
        invocation.try_mutate_checked("abs", |_| Ok(false)).unwrap();
        assert!(invocation.try_mutate_checked("sqrt", |_| Err("reject".into())).is_err());
        assert!(invocation.capture_writes.as_ref().unwrap().1.is_empty());
        assert!(!invocation.scopes[0].contains_key("abs"));
        invocation.mutate_checked("abs", |value| *value = Value::Int(11)).unwrap();
        invocation
            .try_mutate_checked("sqrt", |value| {
                *value = Value::Int(12);
                Ok(true)
            })
            .unwrap();
        env.merge_capture_writes(&invocation);
        assert!(matches!(env.get("abs"), Some(Value::Int(11))));
        assert!(matches!(env.get("sqrt"), Some(Value::Int(12))));
        let mut another = compact();
        another.push_scope();
        another.set("abs".into(), Value::Int(13));
        assert!(another.mutate("sqrt", |value| *value = Value::Int(14)));
        another.pop_scope();
        assert!(matches!(another.get("abs"), Some(Value::Int(13))));
        assert!(matches!(another.get("sqrt"), Some(Value::Int(14))));
    }

    #[test]
    fn compact_captures_keep_exactly_the_same_visible_names() {
        let plain = super::super::Interpreter::new().env;
        let mut compact = plain.clone();
        compact.share_module_native_defaults();
        for source in ["return abs(-1)", "return sqrt(abs(-4))", "return 7"] {
            let body = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap()).parse();
            let before = plain.capture_for_callable(&[], &body, None).visible_bindings();
            let after = compact.capture_for_callable(&[], &body, None).visible_bindings();
            let mut before_names = before.keys().cloned().collect::<Vec<_>>();
            let mut after_names = after.keys().cloned().collect::<Vec<_>>();
            before_names.sort();
            after_names.sort();
            assert_eq!(before_names, after_names, "{source}");
        }
    }

    #[test]
    fn native_defaults_follow_global_swaps_and_preserve_aliases() {
        let mut env = compact();
        assert!(matches!(env.get("println"), Some(Value::NativeFunction(name)) if name == "print"));
        let mut empty = Environment::new();
        env.swap_global_scope(&mut empty);
        assert!(env.get("abs").is_none());
        assert!(matches!(empty.get("abs"), Some(Value::NativeFunction(_))));
        let snapshot = empty.global_snapshot();
        assert!(matches!(snapshot.get("abs"), Some(Value::NativeFunction(_))));
        let mut overridden = super::super::Interpreter::new().env;
        overridden.define("abs".into(), Value::NativeFunction("sqrt".into()));
        overridden.define_with_kind(
            "floor".into(),
            Value::NativeFunction("floor".into()),
            BindingKind::Const,
        );
        overridden.share_module_native_defaults();
        assert!(
            matches!(overridden.get("abs"), Some(Value::NativeFunction(name)) if name == "sqrt")
        );
        assert!(overridden.assign_checked("floor".into(), Value::Null).is_err());
        assert!(overridden.scopes[0].contains_key("abs"));
        assert!(overridden.scopes[0].contains_key("floor"));
    }
}
