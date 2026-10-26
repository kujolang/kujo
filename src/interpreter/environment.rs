// File: src/interpreter/environment.rs
//
// Lexical scoping environment for variable management in the Kujo interpreter.
// Implements a stack of scopes where inner scopes shadow outer scopes.

use super::value::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone, Debug)]
struct LiveCapture {
    owner: Weak<Mutex<Environment>>,
    limit: usize,
}

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

// Most captured calls write zero or one binding. Keep that journal inline;
// promote to a hash set only when an invocation writes distinct bindings.
#[derive(Clone, Debug, Default)]
enum CaptureWrites {
    #[default]
    Empty,
    One((usize, String)),
    Many(HashSet<(usize, String)>),
}

impl CaptureWrites {
    fn record(&mut self, index: usize, name: &str) {
        if matches!(self, Self::One((scope, existing)) if *scope == index && existing == name) {
            return;
        }
        self.insert((index, name.to_owned()));
    }

    fn insert(&mut self, write: (usize, String)) {
        match self {
            Self::Empty => *self = Self::One(write),
            Self::One(existing) if *existing == write => {}
            Self::One(_) => {
                let Self::One(previous) = std::mem::take(self) else { unreachable!() };
                *self = Self::Many(HashSet::from([previous, write]));
            }
            Self::Many(writes) => {
                writes.insert(write);
            }
        }
    }

    fn iter(&self) -> impl Iterator<Item = &(usize, String)> {
        let (first, rest) = match self {
            Self::Empty => (None, None),
            Self::One(write) => (Some(write), None),
            Self::Many(writes) => (None, Some(writes)),
        };
        first.into_iter().chain(rest.into_iter().flat_map(|writes| writes.iter()))
    }

    fn extend(&mut self, writes: impl IntoIterator<Item = (usize, String)>) {
        for write in writes {
            self.insert(write);
        }
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
    #[cfg(test)]
    fn len(&self) -> usize {
        self.iter().count()
    }
    #[cfg(test)]
    fn contains(&self, write: &(usize, String)) -> bool {
        self.iter().any(|item| item == write)
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
    /// Public captured owners retain actual values. Private active invocations use
    /// owner-backed local views; lexical reads should go through `get`.
    pub scopes: Vec<HashMap<String, Value>>,
    // Module snapshots share only immutable native name metadata. Mutable globals
    // remain ordinary owned maps, including every override of a default.
    native_defaults: Option<std::sync::Arc<HashMap<String, String>>>,
    // Missing entries mean Mutable. Store only immutable exceptions, avoiding
    // duplicate name storage for native preloads and ordinary mutable bindings.
    binding_kinds: Vec<HashMap<String, BindingKind>>,
    global_owner: std::sync::Arc<()>,
    // Lexical provenance survives task snapshots; execution ownership does not.
    global_source: std::sync::Arc<()>,
    live_capture: Option<LiveCapture>,
    capture_writes: Option<(usize, CaptureWrites)>,
    // Allocation-free conservative proof for read-only synchronous activations.
    // Local parameters are outside the captured prefix; unknown transitions
    // invalidate the proof rather than assuming they cannot change captures.
    capture_observation: Option<(usize, bool)>,
}

impl Environment {
    /// Create a private invocation view without replacing publicly exposed owner
    /// values or retaining a second current collection value in the invocation.
    pub(crate) fn capture_activation(owner: &Arc<Mutex<Self>>) -> Self {
        let captured = owner.lock().unwrap_or_else(|p| p.into_inner());
        debug_assert!(captured.live_capture.is_none(), "capture owners must own real values");
        let mut activation = captured.clone();
        if activation.scopes.len() > 1 {
            for scope in activation.scopes.iter_mut().skip(1) {
                for value in scope.values_mut() {
                    *value = Value::Null;
                }
            }
            activation.live_capture =
                Some(LiveCapture { owner: Arc::downgrade(owner), limit: activation.scopes.len() });
        }
        activation
    }

    fn capture_owner(link: &Option<LiveCapture>, index: usize) -> Option<Arc<Mutex<Self>>> {
        link.as_ref()
            .filter(|link| index > 0 && index < link.limit)
            .and_then(|link| link.owner.upgrade())
    }

    fn capture_value(link: &Option<LiveCapture>, index: usize, name: &str) -> Option<Value> {
        Self::capture_owner(link, index).and_then(|owner| {
            owner
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .scopes
                .get(index)
                .and_then(|scope| scope.get(name))
                .cloned()
        })
    }

    fn publish_capture_value(
        link: &Option<LiveCapture>,
        index: usize,
        name: &str,
        value: &Value,
    ) -> bool {
        if let Some(owner) = Self::capture_owner(link, index) {
            if let Some(slot) = owner
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .scopes
                .get_mut(index)
                .and_then(|scope| scope.get_mut(name))
            {
                *slot = value.clone();
                return true;
            }
        }
        false
    }

    fn materialized_scopes(&self) -> Vec<HashMap<String, Value>> {
        let mut scopes = self.scopes.clone();
        if let Some(link) = &self.live_capture {
            if let Some(owner) = link.owner.upgrade() {
                let owner = owner.lock().unwrap_or_else(|p| p.into_inner());
                for (index, scope) in scopes.iter_mut().enumerate().take(link.limit).skip(1) {
                    if let Some(shared) = owner.scopes.get(index) {
                        for (name, value) in scope.iter_mut() {
                            if let Some(current) = shared.get(name) {
                                *value = current.clone();
                            }
                        }
                    }
                }
            }
        }
        scopes
    }

    /// Local writes from this activation have already reached the public owner.
    pub(crate) fn has_live_capture(&self) -> bool {
        self.live_capture.is_some()
    }

    pub(super) fn detached_snapshot(&self) -> Self {
        Self {
            scopes: self.materialized_scopes(),
            native_defaults: self.native_defaults.clone(),
            binding_kinds: self.binding_kinds.clone(),
            global_owner: self.global_owner.clone(),
            global_source: self.global_source.clone(),
            live_capture: None,
            capture_writes: self.capture_writes.clone(),
            capture_observation: self.capture_observation,
        }
    }

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
            global_source: self.global_source.clone(),
            live_capture: self.live_capture.clone(),
            capture_writes: self.capture_writes.clone(),
            capture_observation: self.capture_observation,
        }
    }

    pub(crate) fn copy_global_scope_from(&mut self, source: &Self) {
        self.invalidate_capture_observation();
        self.global_source.clone_from(&source.global_source);
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
        if self.live_capture.is_some() {
            for (target, source) in
                self.scopes[1..count].iter_mut().zip(&completed.scopes[1..count])
            {
                *target = source.keys().map(|name| (name.clone(), Value::Null)).collect();
            }
        } else {
            self.scopes[1..count].clone_from_slice(&completed.scopes[1..count]);
        }
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

    /// Track writes for captured invocations inside language tasks. Ordinary environments
    /// allocate no journal. Same-binding read-modify-write remains non-atomic.
    pub(crate) fn track_capture_writes(&mut self) {
        self.capture_writes = Some((self.scopes.len(), CaptureWrites::default()));
    }

    #[cfg(test)]
    pub(crate) fn merge_capture_writes(&mut self, after: &Self) {
        self.merge_capture_writes_from(after, 0);
    }

    fn merge_capture_writes_from(&mut self, after: &Self, first_scope: usize) {
        let Some((_, writes)) = &after.capture_writes else {
            return;
        };
        for (index, name) in writes.iter().filter(|(index, _)| {
            *index >= first_scope && (*index == 0 || after.live_capture.is_none())
        }) {
            if let (Some(target), Some(value)) = (
                self.scopes.get_mut(*index),
                after.scopes.get(*index).and_then(|scope| scope.get(name)),
            ) {
                if let Some(existing) = target.get_mut(name) {
                    *existing = value.clone();
                } else {
                    target.insert(name.clone(), value.clone());
                }
            }
        }
    }

    fn record_capture_write(
        journal: &mut Option<(usize, CaptureWrites)>,
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
                writes.record(index, name);
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
            global_source: std::sync::Arc::new(()),
            live_capture: None,
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
        if self.live_capture.is_some() {
            return self.detached_snapshot().capture_for_callable(params, body, lexical_self);
        }
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
            global_source: self.global_source.clone(),
            live_capture: None,
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
        if self.live_capture.is_some() {
            return self.detached_snapshot().visible_bindings();
        }
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

    pub(crate) fn shares_global_source(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.global_source, &other.global_source)
    }

    /// Publish captured locals, keeping task globals isolated from future invocations.
    pub(crate) fn merge_task_capture_writes(&mut self, after: &Self) {
        self.merge_capture_writes_from(after, 1);
    }

    pub(crate) fn global_snapshot(&self) -> Self {
        Self {
            scopes: vec![self.scopes[0].clone()],
            binding_kinds: vec![self.binding_kinds[0].clone()],
            native_defaults: self.native_defaults.clone(),
            global_owner: std::sync::Arc::new(()),
            global_source: self.global_source.clone(),
            live_capture: None,
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
            global_source: self.global_source.clone(),
            live_capture: self.live_capture.take(),
            capture_writes: self
                .capture_writes
                .as_ref()
                .map(|(limit, _)| (*limit, CaptureWrites::default())),
            capture_observation: None,
        };
        // Helper locals replace suspended caller indices; only globals belong
        // to the active capture journal until the caller scopes are restored.
        if let Some((limit, _)) = &mut self.capture_writes {
            *limit = 1;
        }
        self.push_scope();
        caller
    }

    pub(crate) fn leave_global_function(&mut self, caller: Environment) {
        self.live_capture = caller.live_capture;
        self.scopes.truncate(1);
        self.binding_kinds.truncate(1);
        self.scopes.extend(caller.scopes);
        self.binding_kinds.extend(caller.binding_kinds);
        if let (Some((limit, _)), Some((saved_limit, _))) =
            (&mut self.capture_writes, caller.capture_writes)
        {
            *limit = saved_limit;
        }
    }

    /// Swap only non-global scopes when suspending an execution owner. Global
    /// bindings remain live; caller scopes never leak into a resumed generator.
    pub(crate) fn swap_local_scopes(&mut self, mut owner: Environment) -> Environment {
        let previous = Environment {
            scopes: self.scopes.split_off(1),
            binding_kinds: self.binding_kinds.split_off(1),
            native_defaults: self.native_defaults.clone(),
            global_owner: self.global_owner.clone(),
            global_source: self.global_source.clone(),
            live_capture: self.live_capture.take(),
            capture_writes: None,
            capture_observation: None,
        };
        self.live_capture = owner.live_capture.take();
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
            global_source: std::sync::Arc::new(()),
            live_capture: None,
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
            global_source: self.global_source.clone(),
            live_capture: None,
            capture_writes: None,
            capture_observation: None,
        };
        if capture_locals {
            result.scopes.extend(self.materialized_scopes().into_iter().skip(1));
            result.binding_kinds.extend(self.binding_kinds.iter().skip(1).cloned());
        }
        result
    }

    /// Move the live globals into/out of a continuation without cloning them.
    pub(crate) fn swap_global_scope(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.global_source, &mut other.global_source);
        std::mem::swap(&mut self.native_defaults, &mut other.native_defaults);
        std::mem::swap(&mut self.scopes[0], &mut other.scopes[0]);
        std::mem::swap(&mut self.binding_kinds[0], &mut other.binding_kinds[0]);
    }

    /// Get a variable from the environment, searching from inner to outer scopes
    /// Returns a cloned value if found
    pub fn get(&self, name: &str) -> Option<Value> {
        // Search from innermost to outermost scope
        for (index, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(value) = scope.get(name) {
                if self.live_capture.as_ref().is_some_and(|link| index > 0 && index < link.limit) {
                    return Self::capture_value(&self.live_capture, index, name);
                }
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
        let index = self.scopes.len().saturating_sub(1);
        let shared = if let Some(owner) = Self::capture_owner(&self.live_capture, index) {
            let mut owner = owner.lock().unwrap_or_else(|p| p.into_inner());
            owner.scopes[index].insert(name.clone(), value.clone());
            if kind == BindingKind::Mutable {
                owner.binding_kinds[index].remove(&name);
            } else {
                owner.binding_kinds[index].insert(name.clone(), kind);
            }
            true
        } else {
            false
        };
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.clone(), if shared { Value::Null } else { value });
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
                let shared = Self::publish_capture_value(&self.live_capture, index, &name, &value);
                scope.insert(name, if shared { Value::Null } else { value });
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
                let shared =
                    Self::publish_capture_value(&self.live_capture, scope_index, &name, &value);
                scope.insert(name, if shared { Value::Null } else { value });
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
                if let Some(current) = Self::capture_value(&self.live_capture, index, name) {
                    *value = current;
                }
                f(value);
                if Self::publish_capture_value(&self.live_capture, index, name, value) {
                    *value = Value::Null;
                }
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
                if let Some(current) = Self::capture_value(&self.live_capture, scope_index, name) {
                    *value = current;
                }
                f(value);
                if Self::publish_capture_value(&self.live_capture, scope_index, name, value) {
                    *value = Value::Null;
                }
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

    /// Runtime-only leaf mutation. The operation must not execute Kujo code,
    /// inspect another environment, or invoke host callbacks while holding a cell.
    /// Public host mutation callbacks use `mutate_checked` outside this lock.
    pub(crate) fn mutate_checked_runtime(
        &mut self,
        name: &str,
        operation: impl FnOnce(&mut Value),
    ) -> Result<(), String> {
        let index = self.scopes.iter().rposition(|scope| scope.contains_key(name));
        if let Some(index) = index {
            if let Some(owner) = Self::capture_owner(&self.live_capture, index) {
                let mut owner = owner.lock().unwrap_or_else(|p| p.into_inner());
                let kind =
                    owner.binding_kinds[index].get(name).copied().unwrap_or(BindingKind::Mutable);
                if !kind.allows_mutation() {
                    return Err(kind.mutation_error(name));
                }
                if let Some(value) =
                    owner.scopes.get_mut(index).and_then(|scope| scope.get_mut(name))
                {
                    Self::record_capture_write(
                        &mut self.capture_writes,
                        &mut self.capture_observation,
                        index,
                        name,
                    );
                    operation(value);
                    return Ok(());
                }
            }
        }
        self.mutate_checked(name, operation)
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
                let current = Self::capture_value(&self.live_capture, scope_index, name);
                let shared = current.is_some();
                if let Some(current) = current {
                    *value = current;
                }
                let result = f(value);
                if shared {
                    if matches!(result, Ok(true)) {
                        Self::publish_capture_value(&self.live_capture, scope_index, name, value);
                    }
                    *value = Value::Null;
                }
                let changed = result?;
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
    fn namespace_transfers_do_not_transfer_generator_execution_ownership() {
        let original = Environment::new();
        let mut task = original.global_snapshot();
        let task_owner = task.global_owner();
        let mut foreign = Environment::new();
        let foreign_owner = foreign.global_owner();
        foreign.copy_global_scope_from(&task);
        assert!(foreign.shares_global_source(&original));
        assert!(std::sync::Arc::ptr_eq(&foreign.global_owner(), &foreign_owner));
        assert!(!std::sync::Arc::ptr_eq(&foreign.global_owner(), &task_owner));
        foreign.swap_global_scope(&mut task);
        assert!(std::sync::Arc::ptr_eq(&task.global_owner(), &task_owner));
        assert!(std::sync::Arc::ptr_eq(&foreign.global_owner(), &foreign_owner));
        let empty = Environment::empty_scopes();
        assert!(!empty.shares_global_source(&original));
        assert!(!std::sync::Arc::ptr_eq(&empty.global_owner(), &original.global_owner()));
    }

    #[test]
    fn task_snapshots_keep_lexical_provenance_but_isolate_global_writes() {
        let mut owner = Environment::new();
        owner.define("global".into(), Value::Int(1));
        owner.push_scope();
        owner.define("local".into(), Value::Int(2));
        let snapshot = owner.global_snapshot();
        assert!(owner.shares_global_source(&snapshot));
        assert!(!std::sync::Arc::ptr_eq(&owner.global_owner(), &snapshot.global_owner()));
        assert!(!owner.shares_global_source(&Environment::new()));
        assert_eq!(snapshot.scopes.len(), 1);
        let mut task = owner.clone();
        task.track_capture_writes();
        task.assign_checked("global".into(), Value::Int(9)).unwrap();
        task.assign_checked("local".into(), Value::Int(3)).unwrap();
        owner.merge_task_capture_writes(&task);
        assert!(matches!(owner.get("global"), Some(Value::Int(1))));
        assert!(matches!(owner.get("local"), Some(Value::Int(3))));
        let mut other = Environment::new();
        other.swap_global_scope(&mut task);
        assert!(other.shares_global_source(&owner));
        assert!(!task.shares_global_source(&owner));
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

#[cfg(test)]
mod helper_capture_journal_tests {
    use super::*;

    #[test]
    fn nested_helpers_preserve_actual_writes_and_exclude_parameter_scopes() {
        let mut owner = Environment::new();
        owner.define("global".into(), Value::Int(0));
        owner.push_scope();
        owner.define("untouched".into(), Value::Int(0));
        owner.define("changed".into(), Value::Int(0));
        let mut invocation = owner.clone();
        invocation.track_capture_writes();
        invocation.push_scope();
        let caller = invocation.enter_global_function();
        invocation.define("untouched".into(), Value::Int(12));
        let helper = invocation.enter_global_function();
        invocation.define("untouched".into(), Value::Int(34));
        invocation.assign_checked("global".into(), Value::Int(9)).unwrap();
        invocation.leave_global_function(helper);
        assert!(matches!(invocation.get("untouched"), Some(Value::Int(12))));
        invocation.leave_global_function(caller);
        invocation.assign_checked("changed".into(), Value::Int(1)).unwrap();
        invocation.pop_scope();
        owner.assign_checked("untouched".into(), Value::Int(42)).unwrap();
        owner.merge_capture_writes(&invocation);
        assert!(matches!(owner.get("untouched"), Some(Value::Int(42))));
        assert!(matches!(owner.get("changed"), Some(Value::Int(1))));
        assert!(matches!(owner.get("global"), Some(Value::Int(9))));
        let (limit, writes) = invocation.capture_writes.as_ref().unwrap();
        assert_eq!(*limit, 2);
        assert_eq!(writes.len(), 2);
        assert!(writes.contains(&(0, "global".into())));
        assert!(writes.contains(&(1, "changed".into())));
    }
}

#[cfg(test)]
mod capture_journal_scope_tests {
    use super::{Environment, Value};

    #[test]
    fn repeated_and_shadowed_writes_merge_without_clobbering_other_bindings() {
        let mut owner = Environment::new();
        owner.define("value".into(), Value::Int(0));
        owner.push_scope();
        owner.define("value".into(), Value::Int(1));
        owner.define("other".into(), Value::Int(2));
        let mut invocation = owner.clone();
        invocation.track_capture_writes();
        invocation.assign_checked("value".into(), Value::Int(3)).unwrap();
        invocation.assign_checked("value".into(), Value::Int(4)).unwrap();
        let caller = invocation.enter_global_function();
        invocation.assign_checked("value".into(), Value::Int(5)).unwrap();
        invocation.leave_global_function(caller);
        let mut nested = invocation.clone();
        nested.assign_checked("other".into(), Value::Int(6)).unwrap();
        invocation.merge_capture_writes(&nested);
        owner.merge_capture_writes(&nested);
        assert!(matches!(owner.get("value"), Some(Value::Int(4))));
        assert!(matches!(owner.get("other"), Some(Value::Int(6))));
        owner.pop_scope();
        assert!(matches!(owner.get("value"), Some(Value::Int(5))));
        assert_eq!(nested.capture_writes.as_ref().unwrap().1.len(), 3);
        assert_eq!(invocation.capture_writes.as_ref().unwrap().1.len(), 2);
    }
}

#[cfg(test)]
mod live_capture_storage_tests {
    use super::*;

    fn captured() -> (Arc<Mutex<Environment>>, Environment) {
        let mut env = Environment::new();
        env.define("global".into(), Value::Int(1));
        env.push_scope();
        env.define("items".into(), Value::Array(Arc::new(Vec::with_capacity(256))));
        env.define_with_kind("fixed".into(), Value::Int(7), BindingKind::LetImmutable);
        let owner = Arc::new(Mutex::new(env));
        let view = Environment::capture_activation(&owner);
        (owner, view)
    }

    #[test]
    fn live_capture_definitions_publish_values_and_binding_kinds() {
        let (owner, mut env) = captured();
        env.define_with_kind("new_binding".into(), Value::Int(9), BindingKind::Const);
        assert!(matches!(env.get("new_binding"), Some(Value::Int(9))));
        {
            let owner = owner.lock().unwrap();
            assert!(matches!(owner.scopes[1].get("new_binding"), Some(Value::Int(9))));
            assert_eq!(owner.binding_kinds[1].get("new_binding"), Some(&BindingKind::Const));
        }
        let mut alias = Environment::capture_activation(&owner);
        assert!(alias.assign_checked("new_binding".into(), Value::Int(10)).is_err());
        env.define("new_binding".into(), Value::Int(11));
        let mut alias = Environment::capture_activation(&owner);
        alias.assign_checked("new_binding".into(), Value::Int(12)).unwrap();
        assert!(matches!(env.get("new_binding"), Some(Value::Int(12))));
    }

    #[test]
    fn live_capture_leaf_mutation_keeps_one_collection_owner() {
        let (_owner, env) = captured();
        let mut alias = env.clone();
        let mut storage = None;
        for n in 0..128 {
            alias
                .mutate_checked_runtime("items", |value| {
                    let Value::Array(items) = value else { panic!("expected array") };
                    assert_eq!(
                        Arc::strong_count(items),
                        1,
                        "activation must not retain a second value"
                    );
                    let pointer = Arc::as_ptr(items);
                    if let Some(previous) = storage {
                        assert_eq!(pointer, previous);
                    }
                    storage = Some(pointer);
                    Arc::make_mut(items).push(Value::Int(n));
                })
                .unwrap();
        }
        let Some(Value::Array(items)) = env.get("items") else { panic!("expected array") };
        assert_eq!(items.len(), 128);
    }

    #[test]
    fn live_capture_snapshots_globals_and_immutable_guards_remain_separate() {
        let (_owner, env) = captured();
        let snapshot = env.detached_snapshot();
        let mut alias = env.clone();
        alias.assign_checked("global".into(), Value::Int(9)).unwrap();
        alias
            .mutate_checked_runtime("items", |value| {
                let Value::Array(items) = value else { panic!("expected array") };
                Arc::make_mut(items).push(Value::Int(9));
            })
            .unwrap();
        assert!(matches!(env.get("global"), Some(Value::Int(1))));
        assert!(matches!(snapshot.get("items"), Some(Value::Array(items)) if items.is_empty()));
        assert!(matches!(env.get("items"), Some(Value::Array(items)) if items.len() == 1));
        assert!(alias
            .mutate_checked_runtime("fixed", |_| panic!("immutable mutation ran"))
            .is_err());
        assert!(matches!(env.get("fixed"), Some(Value::Int(7))));
    }

    #[test]
    fn live_capture_host_mutation_does_not_hold_the_shared_lock() {
        let (_owner, env) = captured();
        let mut alias = env.clone();
        alias
            .mutate_checked("items", |value| {
                assert!(matches!(env.get("fixed"), Some(Value::Int(7))));
                let Value::Array(items) = value else { panic!("expected array") };
                Arc::make_mut(items).push(Value::Int(9));
            })
            .unwrap();
        assert!(matches!(env.get("items"), Some(Value::Array(items)) if items.len() == 1));
        let cells = Arc::downgrade(&_owner);
        drop(alias);
        drop(env);
        drop(_owner);
        assert!(cells.upgrade().is_none(), "shared storage outlived its owners");
    }

    #[test]
    fn live_capture_helper_parameters_do_not_alias_suspended_locals() {
        let (_owner, mut env) = captured();
        let alias = env.clone();
        let caller = env.enter_global_function();
        env.define("items".into(), Value::Int(3));
        env.assign_checked("items".into(), Value::Int(4)).unwrap();
        assert!(matches!(alias.get("items"), Some(Value::Array(items)) if items.is_empty()));
        env.leave_global_function(caller);
        env.mutate_checked_runtime("items", |value| {
            let Value::Array(items) = value else { panic!("expected array") };
            Arc::make_mut(items).push(Value::Int(9));
        })
        .unwrap();
        assert!(matches!(alias.get("items"), Some(Value::Array(items)) if items.len() == 1));
    }
    #[test]
    fn rejected_live_capture_mutations_release_temporary_values() {
        let (_owner, mut env) = captured();
        env.track_capture_writes();
        assert!(!env.try_mutate_checked("items", |_| Ok(false)).unwrap());
        assert!(env.try_mutate_checked("items", |_| Err("rejected".into())).is_err());
        assert!(env.capture_writes.as_ref().unwrap().1.is_empty());
        env.mutate_checked_runtime("items", |value| {
            let Value::Array(items) = value else { panic!("expected array") };
            assert_eq!(Arc::strong_count(items), 1);
        })
        .unwrap();
    }
}
