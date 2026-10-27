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
    Indexed(super::indexed_publication::IndexedPublication),
    Dictionary(super::dictionary_publication::DictionaryPublication),
}

#[cfg(test)]
#[test]
fn indexed_publication_candidate_fits_capture_storage_layout() {
    // Compare against the actual existing enum, including its discriminant and
    // enclosing Option. Payload-size comparison alone misses niche/layout costs.
    // Preserve the pre-integration enum layout, not just its largest payload.
    #[allow(dead_code)]
    enum Previous {
        Empty,
        One((usize, String)),
        Many(HashSet<(usize, String)>),
    }
    assert_eq!(std::mem::size_of::<Previous>(), std::mem::size_of::<CaptureWrites>());
    assert_eq!(std::mem::align_of::<Previous>(), std::mem::align_of::<CaptureWrites>());
    assert_eq!(
        std::mem::size_of::<Option<(usize, Previous)>>(),
        std::mem::size_of::<Option<(usize, CaptureWrites)>>()
    );
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
            Self::Indexed(_) | Self::Dictionary(_) => {
                unreachable!("private publication is not a task write journal")
            }
        }
    }

    fn iter(&self) -> impl Iterator<Item = &(usize, String)> {
        let (first, rest) = match self {
            Self::Empty | Self::Indexed(_) | Self::Dictionary(_) => (None, None),
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

    fn publication_name(&self) -> Option<&str> {
        match self {
            Self::Indexed(pending) => Some(pending.name()),
            Self::Dictionary(pending) => Some(pending.name()),
            _ => None,
        }
    }

    fn publication_snapshot(&self) -> Option<Value> {
        match self {
            Self::Indexed(pending) => Some(pending.snapshot_value()),
            Self::Dictionary(pending) => Some(pending.snapshot_value()),
            _ => None,
        }
    }

    fn publish(self, owner: Option<&mut Value>) -> Option<(String, Value)> {
        match self {
            Self::Indexed(pending) => Some(pending.publish(owner)),
            Self::Dictionary(pending) => Some(pending.publish(owner)),
            _ => None,
        }
    }

    fn discard(self) -> Option<(String, Value)> {
        match self {
            Self::Indexed(pending) => Some(pending.discard()),
            Self::Dictionary(pending) => Some(pending.discard()),
            _ => None,
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

/// Recycled private frame storage. Values and environment identities never survive
/// recycling; only map capacity, binding names and binding-kind metadata do.
#[derive(Default)]
struct CaptureStorage {
    scopes: Vec<HashMap<String, Value>>,
    binding_kinds: Vec<HashMap<String, BindingKind>>,
    capacity: usize,
    key_bytes: usize,
}

#[derive(Default)]
pub(crate) struct CaptureStoragePool {
    frames: Vec<CaptureStorage>,
    capacity: usize,
    key_bytes: usize,
}

impl CaptureStoragePool {
    const MAX_FRAMES: usize = 4;
    const MAX_CAPACITY: usize = 8192;
    const MAX_KEY_BYTES: usize = 128 * 1024;
    const MAX_SCOPES: usize = 128;

    fn take(&mut self) -> CaptureStorage {
        let Some(frame) = self.frames.pop() else { return CaptureStorage::default() };
        self.capacity -= frame.capacity;
        self.key_bytes -= frame.key_bytes;
        frame
    }

    pub(crate) fn recycle(&mut self, mut environment: Environment) {
        if let Some((0, pending)) = environment.capture_writes.take() {
            if let Some((name, value)) = pending.discard() {
                environment.scopes[0].insert(name, value);
            }
        }
        if environment.scopes.capacity() > Self::MAX_SCOPES
            || environment.binding_kinds.capacity() > Self::MAX_SCOPES
        {
            return;
        }
        let capacity = environment
            .scopes
            .iter()
            .map(HashMap::capacity)
            .chain(environment.binding_kinds.iter().map(HashMap::capacity))
            .fold(0usize, usize::saturating_add);
        if capacity > Self::MAX_CAPACITY {
            return;
        }
        let key_bytes = environment
            .scopes
            .iter()
            .flat_map(HashMap::keys)
            .chain(environment.binding_kinds.iter().flat_map(HashMap::keys))
            .map(String::capacity)
            .fold(0usize, usize::saturating_add);
        if key_bytes > Self::MAX_KEY_BYTES {
            return;
        }
        // Do not keep collections, functions, host handles, captures or global
        // identities alive in an idle frame. Clear outside the owner lock.
        for scope in &mut environment.scopes {
            for value in scope.values_mut() {
                *value = Value::Null;
            }
        }
        let frame = CaptureStorage {
            scopes: std::mem::take(&mut environment.scopes),
            binding_kinds: std::mem::take(&mut environment.binding_kinds),
            capacity,
            key_bytes,
        };
        drop(environment);
        while self.frames.len() >= Self::MAX_FRAMES
            || self.capacity + capacity > Self::MAX_CAPACITY
            || self.key_bytes + key_bytes > Self::MAX_KEY_BYTES
        {
            let old = self.frames.remove(0);
            self.capacity -= old.capacity;
            self.key_bytes -= old.key_bytes;
        }
        self.capacity += capacity;
        self.key_bytes += key_bytes;
        self.frames.push(frame);
    }
}

// HashMap::clone_from clears/reallocates keys. Refill existing entries instead,
// while consulting the actual source keys/kinds on every invocation. Rust hosts
// may edit the public owner map; no cached shape/version assumption is valid.
fn refill_capture_map<T: Clone>(target: &mut HashMap<String, T>, source: &HashMap<String, T>) {
    // Fresh dense maps can use the normal bulk clone instead of repeatedly
    // growing and rehashing. Do not copy a sparse source's unused buckets or
    // discard an already allocated destination table just to take this path.
    if target.capacity() == 0 && source.capacity() <= source.len().saturating_mul(2) {
        target.clone_from(source);
        return;
    }
    target.retain(|name, _| source.contains_key(name));
    target.reserve(source.len().saturating_sub(target.len()));
    for (name, value) in source {
        if let Some(slot) = target.get_mut(name) {
            slot.clone_from(value);
        } else {
            target.insert(name.clone(), value.clone());
        }
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
#[derive(Debug)]
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

impl Clone for Environment {
    fn clone(&self) -> Self {
        let mut scopes = self.scopes.clone();
        if let Some((name, value)) = self.pending_snapshot_entry() {
            scopes[0].insert(name, value);
        }
        Self {
            scopes,
            native_defaults: self.native_defaults.clone(),
            binding_kinds: self.binding_kinds.clone(),
            global_owner: self.global_owner.clone(),
            global_source: self.global_source.clone(),
            live_capture: self.live_capture.clone(),
            capture_writes: self.capture_writes.clone().filter(|(limit, _)| *limit != 0),
            capture_observation: self.capture_observation,
        }
    }
}

impl Environment {
    /// Only synchronous, nonrecursive global captures enable this private state.
    /// A zero limit distinguishes it from task journals (whose limit is >= 1).
    pub(crate) fn enable_indexed_publication(&mut self) {
        if self.capture_writes.is_none() && self.live_capture.is_none() && self.scopes.len() == 1 {
            self.capture_writes = Some((0, CaptureWrites::Empty));
        }
    }

    fn pending_indexed(&self) -> Option<&super::indexed_publication::IndexedPublication> {
        match &self.capture_writes {
            Some((0, CaptureWrites::Indexed(pending))) => Some(pending),
            _ => None,
        }
    }

    fn pending_name(&self) -> Option<&str> {
        match &self.capture_writes {
            Some((0, pending)) => pending.publication_name(),
            _ => None,
        }
    }

    fn pending_snapshot_entry(&self) -> Option<(String, Value)> {
        let Some((0, pending)) = &self.capture_writes else { return None };
        Some((pending.publication_name()?.to_owned(), pending.publication_snapshot()?))
    }

    fn pending_dictionary(&self) -> Option<&super::dictionary_publication::DictionaryPublication> {
        match &self.capture_writes {
            Some((0, CaptureWrites::Dictionary(pending))) => Some(pending),
            _ => None,
        }
    }

    pub(crate) fn pending_visible(&self, name: &str) -> bool {
        self.pending_name() == Some(name)
            && !self.scopes.iter().skip(1).any(|scope| scope.contains_key(name))
    }

    pub(crate) fn flush_indexed_publication(&mut self) {
        if !matches!(self.capture_writes, Some((0, _))) {
            return;
        }
        if let Some((0, pending)) = self.capture_writes.take() {
            if let Some((name, value)) = pending.publish(None) {
                self.scopes[0].insert(name, value);
            }
        }
    }

    pub(crate) fn flush_indexed_name(&mut self, name: &str) {
        if self.pending_visible(name) {
            self.flush_indexed_publication();
        }
    }

    pub(crate) fn publish_indexed_to(&mut self, owner: &mut Self) {
        if !matches!(self.capture_writes, Some((0, _))) {
            return;
        }
        if let Some((0, pending)) = self.capture_writes.take() {
            let slot = pending.publication_name().and_then(|name| owner.scopes[0].get_mut(name));
            if let Some((name, value)) = pending.publish(slot) {
                self.scopes[0].insert(name, value);
            }
        }
    }

    pub(crate) fn indexed_publication_read(&self, name: &str, index: i64) -> Option<Value> {
        if !self.pending_visible(name) {
            return None;
        }
        if let Some(pending) = self.pending_dictionary() {
            return Some(
                super::dictionary_publication::with_integer_key(index, |key| pending.get(key))
                    .unwrap_or_else(|| Value::Error(format!("Missing map key: {index}"))),
            );
        }
        let pending = self.pending_indexed()?;
        let resolved = if index < 0 { pending.len() as i64 + index } else { index };
        Some(if resolved < 0 {
            Value::Error(format!("Index out of bounds: {index}"))
        } else {
            pending
                .get(resolved as usize)
                .unwrap_or_else(|| Value::Error(format!("Index out of bounds: {index}")))
        })
    }

    pub(crate) fn try_defer_index_write(
        &mut self,
        name: &str,
        index: &Value,
        value: &Value,
    ) -> bool {
        if !matches!(self.capture_writes, Some((0, _)))
            || !matches!(value, Value::Int(_) | Value::Float(_) | Value::Bool(_) | Value::Null)
            || self.scopes.iter().skip(1).any(|scope| scope.contains_key(name))
            || self.binding_kinds[0].contains_key(name)
            || self.pending_name().is_some_and(|pending| pending != name)
        {
            return false;
        }
        if self.pending_dictionary().is_some()
            || matches!(self.scopes[0].get(name), Some(Value::Dict(_)))
        {
            return self.try_defer_dictionary_write(name, index, value);
        }
        let Value::Int(index) = index else { return false };
        let length = if let Some(pending) = self.pending_indexed() {
            if pending.name() != name {
                return false;
            }
            pending.len()
        } else if let Some(Value::Array(base)) = self.scopes[0].get(name) {
            // The owner and this private view must be the only initial handles.
            if Arc::strong_count(base) != 2 {
                return false;
            }
            base.len()
        } else {
            return false;
        };
        let resolved = if *index < 0 { length as i64 + *index } else { *index };
        if resolved < 0 || resolved as usize >= length {
            return false;
        }
        let index = resolved as usize;
        if let Some((0, CaptureWrites::Indexed(pending))) = &mut self.capture_writes {
            if !pending.try_record(index, value) {
                return false;
            }
        } else {
            let (key, Value::Array(base)) = self.scopes[0].remove_entry(name).unwrap() else {
                unreachable!()
            };
            match super::indexed_publication::IndexedPublication::prepare(key, base, index, value) {
                Ok(pending) => self.capture_writes = Some((0, CaptureWrites::Indexed(pending))),
                Err((key, base)) => {
                    self.scopes[0].insert(key, Value::Array(base));
                    return false;
                }
            }
        }
        self.invalidate_capture_observation();
        true
    }

    pub(crate) fn dictionary_publication_read(&self, name: &str, key: &str) -> Option<Value> {
        if !self.pending_visible(name) {
            return None;
        }
        Some(
            self.pending_dictionary()?
                .get(key)
                .unwrap_or_else(|| Value::Error(format!("Missing map key: {key:?}"))),
        )
    }

    fn try_defer_dictionary_write(&mut self, name: &str, index: &Value, value: &Value) -> bool {
        let Value::Int(value) = value else { return false };
        if self.pending_dictionary().is_none()
            && (!matches!(self.capture_writes, Some((0, CaptureWrites::Empty)))
                || !matches!(self.scopes[0].get(name), Some(Value::Dict(base)) if Arc::strong_count(base) == 2))
        {
            return false;
        }
        match index {
            Value::Str(key) => self.try_defer_dictionary_key_write(name, key, *value),
            Value::Int(index) => super::dictionary_publication::with_integer_key(*index, |key| {
                self.try_defer_dictionary_key_write(name, key, *value)
            }),
            _ => false,
        }
    }

    fn try_defer_dictionary_key_write(&mut self, name: &str, key: &str, value: i64) -> bool {
        if let Some((0, CaptureWrites::Dictionary(pending))) = &mut self.capture_writes {
            if !pending.try_record(key, value) {
                return false;
            }
        } else {
            if !matches!(self.capture_writes, Some((0, CaptureWrites::Empty))) {
                return false;
            }
            let Some(Value::Dict(base)) = self.scopes[0].get(name) else { return false };
            if Arc::strong_count(base) != 2 || !base.contains_key(key) {
                return false;
            }
            let (name, Value::Dict(base)) = self.scopes[0].remove_entry(name).unwrap() else {
                unreachable!()
            };
            match super::dictionary_publication::DictionaryPublication::prepare(
                name, base, key, value,
            ) {
                Ok(pending) => self.capture_writes = Some((0, CaptureWrites::Dictionary(pending))),
                Err((name, base)) => {
                    self.scopes[0].insert(name, Value::Dict(base));
                    return false;
                }
            }
        }
        self.invalidate_capture_observation();
        true
    }

    /// Create a private invocation view without replacing publicly exposed owner
    /// values or retaining a second current collection value in the invocation.
    #[cfg(test)]
    pub(crate) fn capture_activation(owner: &Arc<Mutex<Self>>) -> Self {
        Self::capture_activation_with_pool(owner, &mut CaptureStoragePool::default())
    }

    pub(crate) fn capture_activation_with_pool(
        owner: &Arc<Mutex<Self>>,
        pool: &mut CaptureStoragePool,
    ) -> Self {
        let captured = owner.lock().unwrap_or_else(|p| p.into_inner());
        debug_assert!(captured.live_capture.is_none(), "capture owners must own real values");
        let mut storage = pool.take();
        storage.scopes.resize_with(captured.scopes.len(), HashMap::new);
        storage.binding_kinds.resize_with(captured.binding_kinds.len(), HashMap::new);
        for (index, (target, source)) in storage.scopes.iter_mut().zip(&captured.scopes).enumerate()
        {
            if index == 0 {
                refill_capture_map(target, source);
            } else {
                target.retain(|name, _| source.contains_key(name));
                // Size fresh local views from their keys, not the owner's spare
                // buckets, so oversized captures need only one table allocation.
                if target.capacity() == 0 {
                    target.reserve(source.len());
                }
                for name in source.keys() {
                    if !target.contains_key(name) {
                        target.insert(name.clone(), Value::Null);
                    }
                }
            }
        }
        for (target, source) in storage.binding_kinds.iter_mut().zip(&captured.binding_kinds) {
            refill_capture_map(target, source);
        }
        let limit = captured.scopes.len();
        Self {
            scopes: storage.scopes,
            binding_kinds: storage.binding_kinds,
            native_defaults: captured.native_defaults.clone(),
            global_owner: captured.global_owner.clone(),
            global_source: captured.global_source.clone(),
            live_capture: (limit > 1).then(|| LiveCapture { owner: Arc::downgrade(owner), limit }),
            capture_writes: captured.capture_writes.clone(),
            capture_observation: captured.capture_observation,
        }
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
        if let Some((name, value)) = self.pending_snapshot_entry() {
            scopes[0].insert(name, value);
        }
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
            capture_writes: self.capture_writes.clone().filter(|(limit, _)| *limit != 0),
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

    /// Project only proven read dependencies. No callbacks, writes, tasks or active
    /// reentry may use this path; each selected value comes from the actual owner.
    /// Captured locals remain owner-backed and must not become snapshots.
    pub(crate) fn project_capture_reads(
        &self,
        names: &[String],
        params: &[String],
        primitive: bool,
        pool: &mut CaptureStoragePool,
    ) -> Option<Self> {
        if self.scopes.len() != 1 || self.has_live_capture() {
            return None;
        }
        for name in names.iter().filter(|name| !params.contains(name)) {
            let value = self.get(name)?;
            if primitive && !super::value::is_capture_primitive(&value) {
                return None;
            }
        }
        let mut storage = pool.take();
        storage.scopes.resize_with(1, HashMap::new);
        storage.binding_kinds.resize_with(1, HashMap::new);
        let wanted = |name: &String| names.contains(name) && !params.contains(name);
        storage.scopes[0].retain(|name, _| wanted(name));
        storage.binding_kinds[0].retain(|name, _| wanted(name));
        for name in names.iter().filter(|name| wanted(name)) {
            let value = self.get(name)?;
            if let Some(slot) = storage.scopes[0].get_mut(name) {
                *slot = value;
            } else {
                storage.scopes[0].insert(name.clone(), value);
            }
            let kind = self
                .scopes
                .iter()
                .enumerate()
                .rev()
                .find(|(_, scope)| scope.contains_key(name))
                .and_then(|(index, _)| self.binding_kinds[index].get(name))
                .copied();
            if let Some(kind) = kind {
                storage.binding_kinds[0].insert(name.clone(), kind);
            } else {
                storage.binding_kinds[0].remove(name);
            }
        }
        Some(Self {
            scopes: storage.scopes,
            binding_kinds: storage.binding_kinds,
            native_defaults: None,
            global_owner: self.global_owner.clone(),
            global_source: self.global_source.clone(),
            live_capture: None,
            capture_writes: None,
            capture_observation: None,
        })
    }

    /// Preserve capture scope/kind layout without copying recursive parameters.
    pub(crate) fn capture_prefix_snapshot(
        &self,
        count: usize,
        pool: &mut CaptureStoragePool,
    ) -> Self {
        if self.pending_name().is_some() {
            return self.clone().capture_prefix_snapshot(count, pool);
        }
        let mut storage = pool.take();
        storage.scopes.resize_with(count, HashMap::new);
        storage.binding_kinds.resize_with(count, HashMap::new);
        for (target, source) in storage.scopes.iter_mut().zip(&self.scopes[..count]) {
            refill_capture_map(target, source);
        }
        for (target, source) in storage.binding_kinds.iter_mut().zip(&self.binding_kinds[..count]) {
            refill_capture_map(target, source);
        }
        Self {
            scopes: storage.scopes,
            binding_kinds: storage.binding_kinds,
            native_defaults: self.native_defaults.clone(),
            global_owner: self.global_owner.clone(),
            global_source: self.global_source.clone(),
            live_capture: self.live_capture.clone(),
            capture_writes: self.capture_writes.clone(),
            capture_observation: self.capture_observation,
        }
    }

    pub(crate) fn copy_global_scope_from(&mut self, source: &Self) {
        self.flush_indexed_publication();
        if source.pending_name().is_some() {
            return self.copy_global_scope_from(&source.clone());
        }
        self.invalidate_capture_observation();
        self.global_source.clone_from(&source.global_source);
        refill_capture_map(&mut self.scopes[0], &source.scopes[0]);
        refill_capture_map(&mut self.binding_kinds[0], &source.binding_kinds[0]);
        self.native_defaults.clone_from(&source.native_defaults);
        if let (Some((_, target)), Some((_, writes))) =
            (&mut self.capture_writes, &source.capture_writes)
        {
            target.extend(writes.iter().filter(|(index, _)| *index == 0).cloned());
        }
    }

    /// Publish the completed globals at the same boundary as copy_global_scope_from,
    /// then let the caller recycle the retired map rather than reallocating its keys.
    pub(crate) fn publish_global_scope_from(&mut self, source: &mut Self) {
        self.flush_indexed_publication();
        source.publish_indexed_to(self);
        self.invalidate_capture_observation();
        if let (Some((_, target)), Some((_, writes))) =
            (&mut self.capture_writes, &source.capture_writes)
        {
            target.extend(writes.iter().filter(|(index, _)| *index == 0).cloned());
        }
        self.swap_global_scope(source);
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
        self.flush_indexed_publication();
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
        self.pending_visible(name)
            || self.scopes.iter().any(|scope| scope.contains_key(name))
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
        self.flush_indexed_publication();
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
        if self.live_capture.is_some() || self.pending_name().is_some() {
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
        if self.live_capture.is_some() || self.pending_name().is_some() {
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
        if self.pending_name().is_some() {
            return self.clone().global_snapshot();
        }
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
        self.flush_indexed_publication();
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
        self.flush_indexed_publication();
        owner.flush_indexed_publication();
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

    /// Metadata for a direct global-only recursive activation. Its globals are
    /// transferred separately; inherited writes still determine publication.
    pub(crate) fn recursive_global_environment(&self, pool: &mut CaptureStoragePool) -> Self {
        if self.pending_name().is_some() {
            return self.clone().recursive_global_environment(pool);
        }
        let mut storage = pool.take();
        storage.scopes.resize_with(1, HashMap::new);
        storage.binding_kinds.resize_with(1, HashMap::new);
        storage.scopes[0].clear();
        storage.binding_kinds[0].clear();
        Self {
            scopes: storage.scopes,
            binding_kinds: storage.binding_kinds,
            native_defaults: self.native_defaults.clone(),
            global_owner: self.global_owner.clone(),
            global_source: self.global_source.clone(),
            live_capture: self.live_capture.clone(),
            capture_writes: self.capture_writes.clone(),
            capture_observation: self.capture_observation,
        }
    }

    /// Move the live globals into/out of a continuation without cloning them.
    pub(crate) fn swap_global_scope(&mut self, other: &mut Self) {
        self.flush_indexed_publication();
        other.flush_indexed_publication();
        std::mem::swap(&mut self.global_source, &mut other.global_source);
        std::mem::swap(&mut self.native_defaults, &mut other.native_defaults);
        std::mem::swap(&mut self.scopes[0], &mut other.scopes[0]);
        std::mem::swap(&mut self.binding_kinds[0], &mut other.binding_kinds[0]);
    }

    /// Get a variable from the environment, searching from inner to outer scopes
    /// Returns a cloned value if found
    pub fn get(&self, name: &str) -> Option<Value> {
        if self.pending_visible(name) {
            return self
                .capture_writes
                .as_ref()
                .and_then(|(_, pending)| pending.publication_snapshot());
        }
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
        if self.scopes.len() == 1 {
            self.flush_indexed_name(&name);
        }
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
        (self.scopes.len() == 1 && self.pending_visible(name))
            || self.scopes.last().map(|scope| scope.contains_key(name)).unwrap_or(false)
    }

    /// Set an existing variable, searching from inner to outer scopes
    /// If not found, creates it in the current scope
    pub fn set(&mut self, name: String, value: Value) {
        self.flush_indexed_name(&name);
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
        self.flush_indexed_name(&name);
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
        self.flush_indexed_name(name);
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
        self.flush_indexed_name(name);
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
        self.flush_indexed_name(name);
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
        if self.pending_visible(name) {
            return Ok(()); // Admission already checked this global's mutable kind.
        }
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
mod indexed_publication_environment_tests {
    use super::*;

    fn activation() -> (Arc<Mutex<Environment>>, Environment) {
        let mut owner = Environment::new();
        owner.define("data".into(), Value::Array(Arc::new((0..128).map(Value::Int).collect())));
        let owner = Arc::new(Mutex::new(owner));
        let mut active = Environment::capture_activation(&owner);
        active.observe_capture_writes(false);
        active.enable_indexed_publication();
        active.push_scope();
        (owner, active)
    }

    fn element(environment: &Environment, index: usize) -> i64 {
        let Value::Array(values) = environment.get("data").unwrap() else { panic!() };
        let Value::Int(value) = values[index] else { panic!() };
        value
    }

    #[test]
    fn indexed_publication_defers_and_reuses_unique_owner() {
        let (owner, mut active) = activation();
        let pointer = match owner.lock().unwrap().scopes[0].get("data").unwrap() {
            Value::Array(values) => Arc::as_ptr(values),
            _ => unreachable!(),
        };
        assert!(active.try_defer_index_write("data", &Value::Int(-1), &Value::Int(900)));
        assert!(!active.capture_is_unchanged());
        assert!(active.contains_binding("data"));
        assert!(matches!(active.indexed_publication_read("data", -1), Some(Value::Int(900))));
        assert_eq!(element(&owner.lock().unwrap(), 127), 127);
        active.publish_indexed_to(&mut owner.lock().unwrap());
        assert_eq!(element(&active, 127), 900);
        let guard = owner.lock().unwrap();
        let Value::Array(values) = &guard.scopes[0]["data"] else { panic!() };
        assert_eq!(Arc::as_ptr(values), pointer);
        assert_eq!(element(&guard, 127), 900);
    }

    #[test]
    fn indexed_publication_snapshots_and_shadowing_keep_value_semantics() {
        let (owner, mut active) = activation();
        assert!(active.try_defer_index_write("data", &Value::Int(0), &Value::Int(99)));
        for snapshot in [active.clone(), active.detached_snapshot(), active.global_snapshot()] {
            assert_eq!(element(&snapshot, 0), 99);
            assert!(snapshot.pending_indexed().is_none());
        }
        assert!(matches!(active.visible_bindings()["data"].0, Value::Array(_)));
        let observer = owner.lock().unwrap().get("data").unwrap();
        active.define_with_kind("data".into(), Value::Int(8), BindingKind::Const);
        assert!(!active.try_defer_index_write("data", &Value::Int(0), &Value::Int(7)));
        assert!(active.ensure_mutable_for_mutation("data").is_err());
        assert!(active.assign_checked("data".into(), Value::Int(9)).is_err());
        assert!(active.pending_indexed().is_some());
        active.pop_scope();
        active.publish_indexed_to(&mut owner.lock().unwrap());
        let Value::Array(observer) = observer else { panic!() };
        assert!(matches!(observer[0], Value::Int(0)));
        assert_eq!(element(&active, 0), 99);
    }

    #[test]
    fn indexed_publication_fallback_and_task_transition_preserve_writes() {
        let (owner, mut active) = activation();
        assert!(active.try_defer_index_write("data", &Value::Int(0), &Value::Int(10)));
        assert!(active.try_defer_index_write("data", &Value::Int(1), &Value::Int(11)));
        assert!(!active.try_defer_index_write("data", &Value::Int(2), &Value::Int(12)));
        active
            .mutate_checked_runtime("data", |value| {
                let Value::Array(values) = value else { panic!() };
                Arc::make_mut(values)[2] = Value::Int(12);
            })
            .unwrap();
        assert!(active.pending_indexed().is_none());
        assert_eq!(element(&active, 0), 10);
        assert_eq!(element(&active, 2), 12);
        assert_eq!(element(&owner.lock().unwrap(), 0), 0);

        let (_owner, mut active) = activation();
        assert!(active.try_defer_index_write("data", &Value::Int(0), &Value::Int(20)));
        active.track_capture_writes();
        assert!(active.pending_indexed().is_none());
        assert_eq!(element(&active, 0), 20);
        assert!(matches!(active.capture_writes, Some((2, CaptureWrites::Empty))));
    }

    #[test]
    fn dictionary_publication_and_array_writes_cannot_replace_each_others_journal() {
        for dictionary_first in [false, true] {
            let mut owner = Environment::new();
            owner.define(
                "map".into(),
                Value::Dict(Arc::new([(Arc::from("key"), Value::Int(1))].into_iter().collect())),
            );
            owner.define("array".into(), Value::Array(Arc::new(vec![Value::Int(2)])));
            let owner = Arc::new(Mutex::new(owner));
            let mut active = Environment::capture_activation(&owner);
            active.enable_indexed_publication();
            active.push_scope();
            let key = Value::Str(Arc::new("key".into()));
            let writes = if dictionary_first {
                [("map", key), ("array", Value::Int(0))]
            } else {
                [("array", Value::Int(0)), ("map", key)]
            };
            assert!(active.try_defer_index_write(writes[0].0, &writes[0].1, &Value::Int(9)));
            assert!(!active.try_defer_index_write(writes[1].0, &writes[1].1, &Value::Int(9)));
            active
                .mutate_checked_runtime(writes[1].0, |root| {
                    super::super::assignment::update_in_place(
                        root,
                        &[super::super::assignment::Access::Index(writes[1].1.clone())],
                        Value::Int(9),
                    )
                    .unwrap();
                })
                .unwrap();
            assert_eq!(active.pending_name(), Some(writes[0].0));
            let snapshot = active.clone();
            assert!(matches!(
                snapshot.get("map").unwrap().dictionary_get("key"),
                Some(Value::Int(9))
            ));
            assert!(
                matches!(snapshot.get("array"), Some(Value::Array(a)) if matches!(a[0], Value::Int(9)))
            );
            active.pop_scope();
            owner.lock().unwrap().publish_global_scope_from(&mut active);
            let owner = owner.lock().unwrap();
            assert!(matches!(owner.get("map").unwrap().dictionary_get("key"), Some(Value::Int(9))));
            assert!(
                matches!(owner.get("array"), Some(Value::Array(a)) if matches!(a[0], Value::Int(9)))
            );
        }
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

#[cfg(test)]
mod capture_storage_pool_tests {
    use super::*;

    #[test]
    fn recycling_releases_values_and_environment_identities() {
        let mut env = Environment::new();
        let items = Arc::new(vec![Value::Int(7)]);
        let weak_items = Arc::downgrade(&items);
        let weak_identity = Arc::downgrade(&env.global_owner);
        let defaults = Arc::new(HashMap::from([("native".into(), "print".into())]));
        let weak_defaults = Arc::downgrade(&defaults);
        env.native_defaults = Some(defaults);
        env.define("items".into(), Value::Array(items));
        let mut pool = CaptureStoragePool::default();
        pool.recycle(env);
        assert!(weak_items.upgrade().is_none());
        assert!(weak_identity.upgrade().is_none());
        assert!(weak_defaults.upgrade().is_none());
        assert_eq!(pool.frames.len(), 1);
        assert!(matches!(pool.frames[0].scopes[0]["items"], Value::Null));
    }

    #[test]
    fn oversized_capture_views_preserve_live_values_snapshots_and_pool_bounds() {
        let mut initial = Environment::new();
        initial.push_scope();
        initial.scopes[1].reserve(32_768);
        for index in 0..8192 {
            initial.define(format!("extra_{index}"), Value::Int(index));
        }
        let items = Arc::new(vec![Value::Int(1)]);
        initial.define_with_kind("fixed".into(), Value::Array(items.clone()), BindingKind::Const);
        let owner = Arc::new(Mutex::new(initial));
        let mut pool = CaptureStoragePool::default();
        let mut active = Environment::capture_activation_with_pool(&owner, &mut pool);

        assert!(active.scopes[1].values().all(|value| matches!(value, Value::Null)));
        assert_eq!(Arc::strong_count(&items), 2, "private view retained the public collection");
        assert!(active.scopes[1].capacity() < owner.lock().unwrap().scopes[1].capacity());
        let snapshot = active.detached_snapshot();
        owner.lock().unwrap().assign_checked("extra_8191".into(), Value::Int(9001)).unwrap();
        assert!(matches!(active.get("extra_8191"), Some(Value::Int(9001))));
        assert!(matches!(snapshot.get("extra_8191"), Some(Value::Int(8191))));
        active.assign_checked("extra_0".into(), Value::Int(99)).unwrap();
        assert!(matches!(owner.lock().unwrap().get("extra_0"), Some(Value::Int(99))));
        assert!(active.assign_checked("fixed".into(), Value::Null).is_err());
        assert!(matches!(owner.lock().unwrap().get("fixed"), Some(Value::Array(_))));

        pool.recycle(active);
        assert!(pool.frames.is_empty(), "oversized private storage survived in the idle pool");
    }

    #[test]
    fn reused_frames_follow_public_owner_keys_values_and_kinds() {
        let mut initial = Environment::new();
        initial.define("removed".into(), Value::Int(1));
        initial.define_with_kind("changed".into(), Value::Int(2), BindingKind::Const);
        initial.push_scope();
        initial.define("local".into(), Value::Int(3));
        let owner = Arc::new(Mutex::new(initial));
        let mut pool = CaptureStoragePool::default();
        let first = Environment::capture_activation_with_pool(&owner, &mut pool);
        pool.recycle(first);
        {
            let mut original = owner.lock().unwrap();
            original.scopes[0].remove("removed");
            original.scopes[0].insert("changed".into(), Value::Int(20));
            original.binding_kinds[0].remove("changed");
            original.scopes[0].insert("added".into(), Value::Int(4));
            original.scopes[1].remove("local");
            original.scopes[1].insert("new_local".into(), Value::Int(5));
            original.binding_kinds[1].insert("new_local".into(), BindingKind::LetImmutable);
        }
        let mut next = Environment::capture_activation_with_pool(&owner, &mut pool);
        assert!(next.get("removed").is_none());
        assert!(next.get("local").is_none());
        assert!(matches!(next.get("changed"), Some(Value::Int(20))));
        assert!(matches!(next.get("added"), Some(Value::Int(4))));
        assert!(matches!(next.get("new_local"), Some(Value::Int(5))));
        assert!(matches!(next.scopes[1]["new_local"], Value::Null));
        next.assign_checked("changed".into(), Value::Int(21)).unwrap();
        assert!(next.assign_checked("new_local".into(), Value::Int(6)).is_err());
        assert!(matches!(owner.lock().unwrap().scopes[1]["new_local"], Value::Int(5)));
    }

    #[test]
    fn reuse_keeps_existing_name_storage_without_stale_defaults_or_scopes() {
        let mut old = Environment::new();
        old.define("reused".into(), Value::Int(1));
        old.push_scope();
        old.define("stale".into(), Value::Int(2));
        old.native_defaults = Some(Arc::new(HashMap::from([("gone".into(), "print".into())])));
        let key = old.scopes[0].keys().find(|k| k.as_str() == "reused").unwrap().as_ptr();
        let mut pool = CaptureStoragePool::default();
        pool.recycle(old);
        let mut fresh = Environment::new();
        fresh.define("reused".into(), Value::Int(9));
        let owner = Arc::new(Mutex::new(fresh));
        let next = Environment::capture_activation_with_pool(&owner, &mut pool);
        assert_eq!(next.scopes[0].keys().find(|k| k.as_str() == "reused").unwrap().as_ptr(), key);
        assert_eq!(next.scopes.len(), 1);
        assert!(next.native_defaults.is_none());
        assert!(next.get("stale").is_none());
        assert!(matches!(next.get("reused"), Some(Value::Int(9))));
        assert_eq!(pool.capacity, 0);
        assert_eq!(pool.key_bytes, 0);
    }

    #[test]
    fn pool_bounds_retained_frames_maps_and_reserved_key_bytes() {
        let mut pool = CaptureStoragePool::default();
        for _ in 0..10 {
            let mut env = Environment::new();
            env.define("value".into(), Value::Int(1));
            pool.recycle(env);
        }
        assert_eq!(pool.frames.len(), CaptureStoragePool::MAX_FRAMES);
        assert_eq!(pool.capacity, pool.frames.iter().map(|f| f.capacity).sum::<usize>());
        assert_eq!(pool.key_bytes, pool.frames.iter().map(|f| f.key_bytes).sum::<usize>());
        let before = (pool.frames.len(), pool.capacity, pool.key_bytes);
        let mut huge_map = Environment::new();
        huge_map.scopes[0].reserve(CaptureStoragePool::MAX_CAPACITY + 1);
        pool.recycle(huge_map);
        let mut huge_name = Environment::new();
        let mut name = String::with_capacity(CaptureStoragePool::MAX_KEY_BYTES + 1);
        name.push('x');
        huge_name.scopes[0].insert(name, Value::Int(1));
        pool.recycle(huge_name);
        let mut huge_scopes = Environment::new();
        huge_scopes.scopes.reserve(CaptureStoragePool::MAX_SCOPES + 1);
        pool.recycle(huge_scopes);
        assert_eq!((pool.frames.len(), pool.capacity, pool.key_bytes), before);
        while !pool.frames.is_empty() {
            pool.take();
        }
        assert_eq!((pool.capacity, pool.key_bytes), (0, 0));
    }

    #[test]
    fn moving_published_globals_matches_copy_and_retains_local_owner_values() {
        let mut owner = Environment::new();
        owner.define("global".into(), Value::Int(1));
        owner.push_scope();
        owner.define("local".into(), Value::Int(2));
        owner.track_capture_writes();
        let mut completed = owner.clone();
        completed.assign_checked("global".into(), Value::Int(3)).unwrap();
        let mut expected = owner.clone();
        expected.copy_global_scope_from(&completed);
        owner.publish_global_scope_from(&mut completed);
        assert!(matches!(owner.get("global"), Some(Value::Int(3))));
        assert!(matches!(owner.get("local"), Some(Value::Int(2))));
        assert!(matches!(completed.get("global"), Some(Value::Int(1))));
        assert_eq!(
            owner.capture_writes.as_ref().unwrap().1.iter().collect::<HashSet<_>>(),
            expected.capture_writes.as_ref().unwrap().1.iter().collect::<HashSet<_>>()
        );
        assert!(Arc::ptr_eq(&owner.global_source, &expected.global_source));
    }
}

#[cfg(test)]
mod recursive_capture_storage_tests {
    use super::*;

    #[test]
    fn prefix_reuse_excludes_parameters_and_keeps_value_snapshots() {
        let mut source = Environment::new();
        source.define("global".into(), Value::Int(1));
        source.push_scope();
        source.define("items".into(), Value::Array(Arc::new(vec![Value::Int(7)])));
        source.push_scope();
        source.define("parameter".into(), Value::Int(99));
        let mut pool = CaptureStoragePool::default();
        let first = source.capture_prefix_snapshot(2, &mut pool);
        let key = first.scopes[0].keys().find(|k| k.as_str() == "global").unwrap().as_ptr();
        pool.recycle(first);
        source.scopes[0].insert("global".into(), Value::Int(2));
        let mut next = source.capture_prefix_snapshot(2, &mut pool);
        assert_eq!(next.scopes.len(), 2);
        assert!(next.get("parameter").is_none());
        assert!(matches!(next.get("global"), Some(Value::Int(2))));
        assert_eq!(next.scopes[0].keys().find(|k| k.as_str() == "global").unwrap().as_ptr(), key);
        next.assign_checked("global".into(), Value::Int(3)).unwrap();
        next.mutate_checked_runtime("items", |value| {
            let Value::Array(items) = value else { panic!("expected array") };
            Arc::make_mut(items).push(Value::Int(8));
        })
        .unwrap();
        assert!(matches!(source.get("global"), Some(Value::Int(2))));
        assert!(matches!(source.get("items"), Some(Value::Array(items)) if items.len() == 1));
        assert!(matches!(next.get("items"), Some(Value::Array(items)) if items.len() == 2));
    }

    #[test]
    fn recursive_prefix_keeps_live_locals_and_separate_globals() {
        let mut source = Environment::new();
        source.define("global".into(), Value::Int(1));
        source.push_scope();
        source.define("items".into(), Value::Array(Arc::new(vec![Value::Int(7)])));
        source.define_with_kind("fixed".into(), Value::Int(8), BindingKind::Const);
        let owner = Arc::new(Mutex::new(source));
        let activation = Environment::capture_activation(&owner);
        let mut pool = CaptureStoragePool::default();
        let mut recursive = activation.capture_prefix_snapshot(2, &mut pool);
        recursive.assign_checked("global".into(), Value::Int(2)).unwrap();
        assert!(recursive.assign_checked("fixed".into(), Value::Int(9)).is_err());
        recursive
            .mutate_checked_runtime("items", |value| {
                let Value::Array(items) = value else { panic!("expected array") };
                assert_eq!(Arc::strong_count(items), 1);
                Arc::make_mut(items).push(Value::Int(8));
            })
            .unwrap();
        assert!(matches!(recursive.scopes[1]["items"], Value::Null));
        assert!(matches!(activation.get("items"), Some(Value::Array(items)) if items.len() == 2));
        assert!(matches!(owner.lock().unwrap().get("global"), Some(Value::Int(1))));
        pool.recycle(recursive);
        assert_eq!(Arc::strong_count(&owner), 1);
        assert!(matches!(owner.lock().unwrap().scopes[1]["items"], Value::Array(_)));
    }

    #[test]
    fn global_refill_refreshes_shape_and_permissions_without_replacing_owner() {
        let mut target = Environment::new();
        target.define("items".into(), Value::Array(Arc::new(vec![Value::Int(0)])));
        target.define("removed".into(), Value::Int(3));
        target.define("fixed".into(), Value::Int(4));
        target.native_defaults = Some(Arc::new(HashMap::from([("stale".into(), "print".into())])));
        target.push_scope();
        target.define("local".into(), Value::Int(5));
        target.track_capture_writes();
        target.assign_checked("local".into(), Value::Int(6)).unwrap();
        let owner_identity = target.global_owner.clone();
        let key = target.scopes[0].keys().find(|k| k.as_str() == "items").unwrap().as_ptr();
        let mut source = Environment::new();
        source.define("items".into(), Value::Array(Arc::new(vec![Value::Int(7)])));
        source.define_with_kind("fixed".into(), Value::Int(8), BindingKind::Const);
        source.track_capture_writes();
        source.assign_checked("items".into(), Value::Array(Arc::new(vec![Value::Int(9)]))).unwrap();
        target.copy_global_scope_from(&source);
        assert!(Arc::ptr_eq(&target.global_owner, &owner_identity));
        assert!(Arc::ptr_eq(&target.global_source, &source.global_source));
        assert_eq!(target.scopes[0].keys().find(|k| k.as_str() == "items").unwrap().as_ptr(), key);
        assert!(target.get("removed").is_none());
        assert!(target.native_defaults.is_none());
        assert!(matches!(target.get("local"), Some(Value::Int(6))));
        assert!(target.assign_checked("fixed".into(), Value::Int(10)).is_err());
        let writes = &target.capture_writes.as_ref().unwrap().1;
        assert!(writes.contains(&(0, "items".into())));
        assert!(writes.contains(&(1, "local".into())));
        target
            .mutate_checked_runtime("items", |value| {
                let Value::Array(items) = value else { panic!("expected array") };
                Arc::make_mut(items).push(Value::Int(10));
            })
            .unwrap();
        assert!(matches!(source.get("items"), Some(Value::Array(items)) if items.len() == 1));
        assert!(matches!(target.get("items"), Some(Value::Array(items)) if items.len() == 2));
    }
}

#[cfg(test)]
mod sparse_capture_storage_tests {
    use super::*;

    #[test]
    fn cold_sparse_prefix_preserves_values_and_kinds_without_empty_bucket_growth() {
        let mut source = Environment::new();
        source.scopes[0].reserve(8192);
        source.binding_kinds[0].reserve(8192);
        source.define("items".into(), Value::Array(Arc::new(vec![Value::Int(7)])));
        source.define_with_kind("fixed".into(), Value::Int(8), BindingKind::Const);
        let mut pool = CaptureStoragePool::default();
        let mut snapshot = source.capture_prefix_snapshot(1, &mut pool);
        assert!(snapshot.scopes[0].capacity() < source.scopes[0].capacity() / 4);
        assert!(snapshot.binding_kinds[0].capacity() < source.binding_kinds[0].capacity() / 4);
        assert!(snapshot.assign_checked("fixed".into(), Value::Int(9)).is_err());
        snapshot
            .mutate_checked_runtime("items", |value| {
                let Value::Array(items) = value else { panic!("expected array") };
                Arc::make_mut(items).push(Value::Int(9));
            })
            .unwrap();
        assert!(matches!(source.get("items"), Some(Value::Array(items)) if items.len() == 1));
        assert!(matches!(snapshot.get("items"), Some(Value::Array(items)) if items.len() == 2));
        pool.recycle(snapshot);
        assert_eq!(pool.frames.len(), 1, "small snapshots of sparse owners remain cacheable");
    }
}

#[cfg(test)]
mod read_projection_tests {
    use super::*;
    #[test]
    fn captured_locals_keep_live_owner_reads_instead_of_projection_snapshots() {
        let mut environment = Environment::new();
        environment.push_scope();
        environment.define("state".into(), Value::Int(7));
        let owner = Arc::new(Mutex::new(environment));
        let ordinary = Environment::capture_activation(&owner);
        let mut pool = CaptureStoragePool::default();
        let projected =
            owner.lock().unwrap().project_capture_reads(&["state".into()], &[], false, &mut pool);
        owner.lock().unwrap().assign_checked("state".into(), Value::Int(8)).unwrap();
        assert!(matches!(ordinary.get("state"), Some(Value::Int(8))));
        if let Some(snapshot) = &projected {
            assert!(matches!(snapshot.get("state"), Some(Value::Int(7))));
        }
        assert!(projected.is_none(), "projection would hide live captured-local writes");
    }

    #[test]
    fn projected_reads_refresh_actual_values_and_keep_snapshots() {
        let mut owner = Environment::new();
        owner.define("unused".into(), Value::Int(99));
        owner.define_with_kind(
            "state".into(),
            Value::Array(Arc::new(vec![Value::Int(7)])),
            BindingKind::Const,
        );
        let mut pool = CaptureStoragePool::default();
        let names = vec!["state".into(), "index".into()];
        let first =
            owner.project_capture_reads(&names, &["index".into()], false, &mut pool).unwrap();
        assert!(first.get("unused").is_none());
        assert!(first.get("index").is_none());
        assert!(matches!(first.binding_kinds[0].get("state"), Some(BindingKind::Const)));
        let snapshot = first.get("state").unwrap();
        pool.recycle(first);
        owner.scopes[0].insert("state".into(), Value::Array(Arc::new(vec![Value::Int(8)])));
        owner.binding_kinds[0].remove("state");
        let next =
            owner.project_capture_reads(&names, &["index".into()], false, &mut pool).unwrap();
        assert!(next.binding_kinds[0].get("state").is_none());
        assert!(matches!(snapshot, Value::Array(v) if matches!(v[0], Value::Int(7))));
        assert!(
            matches!(next.get("state"), Some(Value::Array(v)) if matches!(v[0], Value::Int(8)))
        );
        assert!(owner.project_capture_reads(&names, &["index".into()], true, &mut pool).is_none());
        assert!(owner.project_capture_reads(&["missing".into()], &[], false, &mut pool).is_none());
    }
}
