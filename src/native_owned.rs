//! Recyclable roots for the experimental native owned-value ABI.
//!
//! A root owns one ordinary `Value`, preserving its existing copy-on-write rules.
//! Slots and lookup capacity follow peak live roots, not the number of loads.
//! Handles are monotonically issued identities, never slot addresses or integer
//! markers. Frame exit releases only that frame's roots; a return must explicitly
//! promote its root or take its Value before exit. Integration with generated
//! native operations is deliberately separate from this lifetime mechanism.
#![allow(dead_code)] // Staged ABI foundation; native boundaries adopt it together.

use crate::interpreter::Value;
use nohash_hasher::BuildNoHashHasher;
use std::collections::HashMap;

pub(crate) const OWNED: i64 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Handle(u64);

impl Handle {
    pub(crate) fn payload(self) -> i64 {
        self.0 as i64
    }

    pub(crate) fn from_payload(payload: i64) -> Self {
        Self(payload as u64)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameToken {
    depth: usize,
    identity: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnershipError {
    NoFrame,
    StaleHandle,
    NotCurrentFrame,
    NoParentFrame,
    FrameOrder,
    IdentityExhausted,
    InvalidKind(i64),
}

struct Frame {
    identity: u64,
    head: Option<usize>,
}

struct Slot {
    value: Option<Value>,
    handle: Handle,
    owner: usize,
    previous: Option<usize>,
    // Next live root in a frame, or next free slot when value is None.
    next: Option<usize>,
}

pub(crate) struct NativeOwnedValues {
    slots: Vec<Slot>,
    free: Option<usize>,
    lookup: HashMap<u64, usize, BuildNoHashHasher<u64>>,
    frames: Vec<Frame>,
    next_handle: Option<u64>,
    next_frame: Option<u64>,
}

/// Native control flow enters scopes even when it uses only immediates. Defer
/// all frame storage until an owned value actually appears on that path.
#[derive(Default)]
pub(crate) struct NativeCallRoots {
    values: NativeOwnedValues,
    depth: usize,
}

impl NativeCallRoots {
    pub(crate) fn depth(&self) -> usize {
        self.depth
    }

    pub(crate) fn enter(&mut self) -> Result<(), OwnershipError> {
        self.depth = self.depth.checked_add(1).ok_or(OwnershipError::IdentityExhausted)?;
        Ok(())
    }

    pub(crate) fn leave(&mut self) -> Result<(), OwnershipError> {
        if self.depth == 0 || self.values.frames.len() > self.depth {
            return Err(OwnershipError::FrameOrder);
        }
        if self.values.frames.len() == self.depth {
            let frame = self.values.frames.last().expect("current root frame");
            let token = FrameToken { depth: self.depth - 1, identity: frame.identity };
            self.values.leave_frame(token)?;
        }
        self.depth -= 1;
        Ok(())
    }

    pub(crate) fn encode(&mut self, value: Value) -> Result<(i64, i64), OwnershipError> {
        if let Some(immediate) = crate::native_scalar::encode(&value) {
            return Ok(immediate);
        }
        if self.depth == 0 {
            return Err(OwnershipError::NoFrame);
        }
        while self.values.frames.len() < self.depth {
            self.values.enter_frame()?;
        }
        self.values.encode(value)
    }

    /// Borrowing a native operand does not consume its root. This also permits
    /// a nested scope to marshal a caller's value without taking ownership.
    pub(crate) fn clone_value(&self, payload: i64, kind: i64) -> Result<Value, OwnershipError> {
        self.values.clone_value(payload, kind)
    }

    pub(crate) fn take_value(&mut self, payload: i64, kind: i64) -> Result<Value, OwnershipError> {
        // An empty callee has a logical scope but no physical root frame yet.
        // Do not let the store mistake the caller's frame for the current one.
        if kind == OWNED && self.values.frames.len() != self.depth {
            return Err(OwnershipError::NotCurrentFrame);
        }
        self.values.take_value(payload, kind)
    }

    pub(crate) fn promote(&mut self, payload: i64, kind: i64) -> Result<(), OwnershipError> {
        if kind == OWNED && self.values.frames.len() != self.depth {
            return Err(OwnershipError::NotCurrentFrame);
        }
        self.values.promote_payload(payload, kind)
    }

    #[cfg(test)]
    pub(crate) fn storage_state(&self) -> (usize, usize, usize, usize) {
        (
            self.depth,
            self.values.lookup.len(),
            self.values.slots.capacity(),
            self.values.frames.capacity(),
        )
    }
}

impl Default for NativeOwnedValues {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            free: None,
            lookup: HashMap::default(),
            frames: Vec::new(),
            next_handle: Some(1),
            next_frame: Some(1),
        }
    }
}

impl NativeOwnedValues {
    /// Immediate values keep the existing allocation-free native ABI. Only
    /// owned values acquire a root in the current frame.
    pub(crate) fn encode(&mut self, value: Value) -> Result<(i64, i64), OwnershipError> {
        match crate::native_scalar::encode(&value) {
            Some(payload) => Ok(payload),
            None => Ok((self.insert(value)?.payload(), OWNED)),
        }
    }

    pub(crate) fn clone_value(&self, payload: i64, kind: i64) -> Result<Value, OwnershipError> {
        match kind {
            0..=3 => Ok(crate::native_scalar::decode(payload, kind)),
            OWNED => Ok(self.get(Handle::from_payload(payload))?.clone()),
            _ => Err(OwnershipError::InvalidKind(kind)),
        }
    }

    pub(crate) fn take_value(&mut self, payload: i64, kind: i64) -> Result<Value, OwnershipError> {
        match kind {
            0..=3 => Ok(crate::native_scalar::decode(payload, kind)),
            OWNED => self.take(Handle::from_payload(payload)),
            _ => Err(OwnershipError::InvalidKind(kind)),
        }
    }

    pub(crate) fn duplicate_payload(
        &mut self,
        payload: i64,
        kind: i64,
    ) -> Result<i64, OwnershipError> {
        match kind {
            0..=3 => Ok(payload),
            OWNED => Ok(self.duplicate(Handle::from_payload(payload))?.payload()),
            _ => Err(OwnershipError::InvalidKind(kind)),
        }
    }

    pub(crate) fn release_payload(
        &mut self,
        payload: i64,
        kind: i64,
    ) -> Result<(), OwnershipError> {
        match kind {
            0..=3 => Ok(()),
            OWNED => self.release(Handle::from_payload(payload)),
            _ => Err(OwnershipError::InvalidKind(kind)),
        }
    }

    pub(crate) fn promote_payload(
        &mut self,
        payload: i64,
        kind: i64,
    ) -> Result<(), OwnershipError> {
        match kind {
            0..=3 => Ok(()),
            OWNED => self.promote(Handle::from_payload(payload)),
            _ => Err(OwnershipError::InvalidKind(kind)),
        }
    }

    fn issue(next: &mut Option<u64>) -> Result<u64, OwnershipError> {
        let identity = next.ok_or(OwnershipError::IdentityExhausted)?;
        *next = identity.checked_add(1);
        Ok(identity)
    }

    pub(crate) fn enter_frame(&mut self) -> Result<FrameToken, OwnershipError> {
        let identity = Self::issue(&mut self.next_frame)?;
        let depth = self.frames.len();
        self.frames.push(Frame { identity, head: None });
        Ok(FrameToken { depth, identity })
    }

    pub(crate) fn leave_frame(&mut self, token: FrameToken) -> Result<(), OwnershipError> {
        if self.frames.len() != token.depth + 1
            || self.frames.last().map(|frame| frame.identity) != Some(token.identity)
        {
            return Err(OwnershipError::FrameOrder);
        }
        while let Some(index) = self.frames[token.depth].head {
            drop(self.remove(index));
        }
        self.frames.pop();
        Ok(())
    }

    pub(crate) fn insert(&mut self, value: Value) -> Result<Handle, OwnershipError> {
        let owner = self.frames.len().checked_sub(1).ok_or(OwnershipError::NoFrame)?;
        let handle = Handle(Self::issue(&mut self.next_handle)?);
        let index = match self.free {
            Some(index) => {
                self.free = self.slots[index].next;
                index
            }
            None => {
                self.slots.push(Slot { value: None, handle, owner, previous: None, next: None });
                self.slots.len() - 1
            }
        };
        self.slots[index].value = Some(value);
        self.slots[index].handle = handle;
        self.link(index, owner);
        self.lookup.insert(handle.0, index);
        Ok(handle)
    }

    fn index(&self, handle: Handle) -> Result<usize, OwnershipError> {
        self.lookup.get(&handle.0).copied().ok_or(OwnershipError::StaleHandle)
    }

    pub(crate) fn get(&self, handle: Handle) -> Result<&Value, OwnershipError> {
        let index = self.index(handle)?;
        Ok(self.slots[index].value.as_ref().expect("live native root"))
    }

    /// Loading an outer frame's value creates a distinct owned root. It cannot
    /// let the callee release the caller's root by sharing the same handle.
    pub(crate) fn duplicate(&mut self, handle: Handle) -> Result<Handle, OwnershipError> {
        self.insert(self.get(handle)?.clone())
    }

    fn current_index(&self, handle: Handle) -> Result<usize, OwnershipError> {
        let index = self.index(handle)?;
        if self.slots[index].owner.checked_add(1) != Some(self.frames.len()) {
            return Err(OwnershipError::NotCurrentFrame);
        }
        Ok(index)
    }

    /// Transfer ownership out of the native representation without cloning.
    pub(crate) fn take(&mut self, handle: Handle) -> Result<Value, OwnershipError> {
        let index = self.current_index(handle)?;
        Ok(self.remove(index))
    }

    pub(crate) fn release(&mut self, handle: Handle) -> Result<(), OwnershipError> {
        drop(self.take(handle)?);
        Ok(())
    }

    /// Move a native return into the suspended caller's frame. The payload is
    /// unchanged and the callee's cleanup will no longer release this root.
    pub(crate) fn promote(&mut self, handle: Handle) -> Result<(), OwnershipError> {
        let index = self.current_index(handle)?;
        let parent = self.frames.len().checked_sub(2).ok_or(OwnershipError::NoParentFrame)?;
        self.unlink(index);
        self.link(index, parent);
        Ok(())
    }

    fn link(&mut self, index: usize, owner: usize) {
        let head = self.frames[owner].head;
        self.slots[index].owner = owner;
        self.slots[index].previous = None;
        self.slots[index].next = head;
        if let Some(head) = head {
            self.slots[head].previous = Some(index);
        }
        self.frames[owner].head = Some(index);
    }

    fn unlink(&mut self, index: usize) {
        let slot = &self.slots[index];
        let (previous, next, owner) = (slot.previous, slot.next, slot.owner);
        match previous {
            Some(previous) => self.slots[previous].next = next,
            None => self.frames[owner].head = next,
        }
        if let Some(next) = next {
            self.slots[next].previous = previous;
        }
    }

    fn remove(&mut self, index: usize) -> Value {
        self.unlink(index);
        let slot = &mut self.slots[index];
        self.lookup.remove(&slot.handle.0);
        let value = slot.value.take().expect("live native root");
        slot.previous = None;
        slot.next = self.free;
        self.free = Some(index);
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn lazy_scopes_do_not_allocate_for_immediates_or_release_outer_roots() {
        let mut roots = NativeCallRoots::default();
        for _ in 0..1000 {
            roots.enter().unwrap();
            roots.enter().unwrap();
            roots.encode(Value::Float(3.5)).unwrap();
            roots.leave().unwrap();
            roots.leave().unwrap();
        }
        assert_eq!(roots.storage_state(), (0, 0, 0, 0));
        roots.enter().unwrap();
        let (payload, kind) = roots.encode(Value::Str(Arc::new("caller".into()))).unwrap();
        roots.enter().unwrap();
        assert_eq!(roots.take_value(payload, kind).unwrap_err(), OwnershipError::NotCurrentFrame);
        roots.leave().unwrap();
        assert!(
            matches!(roots.take_value(payload, kind).unwrap(), Value::Str(text) if text.as_str() == "caller")
        );
        roots.leave().unwrap();
        assert_eq!(roots.depth(), 0);
        assert_eq!(roots.leave(), Err(OwnershipError::FrameOrder));
    }

    #[test]
    fn immediate_codec_never_allocates_roots_or_changes_payload_bits() {
        let mut values = NativeOwnedValues::default();
        for _ in 0..1000 {
            for original in [
                Value::Int(-1),
                Value::Int(i64::MIN),
                Value::Int(i64::MAX),
                Value::Null,
                Value::Bool(false),
                Value::Bool(true),
                Value::Float(-0.0),
                Value::Float(f64::INFINITY),
                Value::Float(f64::from_bits(0x7ff8_0000_0000_0042)),
            ] {
                let expected = crate::native_scalar::encode(&original).unwrap();
                let (payload, kind) = values.encode(original).unwrap();
                assert_eq!((payload, kind), expected);
                assert_eq!(values.duplicate_payload(payload, kind).unwrap(), payload);
                let borrowed = values.clone_value(payload, kind).unwrap();
                let taken = values.take_value(payload, kind).unwrap();
                assert_eq!(crate::native_scalar::encode(&borrowed), Some(expected));
                assert_eq!(crate::native_scalar::encode(&taken), Some(expected));
                values.release_payload(payload, kind).unwrap();
                values.promote_payload(payload, kind).unwrap();
            }
        }
        assert_eq!(values.slots.capacity(), 0);
        assert_eq!(values.lookup.capacity(), 0);
        assert_eq!(values.frames.capacity(), 0);
        assert_eq!(values.next_handle, Some(1));
    }

    #[test]
    fn owned_codec_uses_explicit_kinds_and_preserves_callback_return_ownership() {
        let mut values = NativeOwnedValues::default();
        let caller = values.enter_frame().unwrap();
        let bytes = Arc::new(vec![0, 128, 255]);
        let (argument, kind) = values.encode(Value::Bytes(bytes.clone())).unwrap();
        assert_eq!(kind, OWNED);
        let callee = values.enter_frame().unwrap();
        let result = values.duplicate_payload(argument, kind).unwrap();
        values.promote_payload(result, kind).unwrap();
        values.leave_frame(callee).unwrap();
        values.release_payload(argument, kind).unwrap();
        let Value::Bytes(returned) = values.take_value(result, kind).unwrap() else { panic!() };
        assert!(Arc::ptr_eq(&returned, &bytes));
        assert_eq!(values.clone_value(result, kind).unwrap_err(), OwnershipError::StaleHandle);
        for bad_kind in [-1, 5, i64::MAX] {
            assert_eq!(
                values.clone_value(0, bad_kind).unwrap_err(),
                OwnershipError::InvalidKind(bad_kind)
            );
            assert_eq!(
                values.release_payload(0, bad_kind),
                Err(OwnershipError::InvalidKind(bad_kind))
            );
        }
        values.leave_frame(caller).unwrap();
        assert_eq!(Arc::strong_count(&bytes), 2);
    }

    #[test]
    fn recycled_slots_reject_stale_handles_and_preserve_all_payload_bits() {
        let mut values = NativeOwnedValues::default();
        let frame = values.enter_frame().unwrap();
        let first = values.insert(Value::Int(1)).unwrap();
        values.release(first).unwrap();
        values.next_handle = Some(u64::MAX);
        let last = values.insert(Value::Int(2)).unwrap();
        assert_eq!(last.payload(), -1);
        assert_eq!(Handle::from_payload(last.payload()), last);
        assert!(matches!(values.clone_value(-1, OWNED).unwrap(), Value::Int(2)));
        assert!(matches!(
            values.clone_value(-1, crate::native_scalar::INTEGER).unwrap(),
            Value::Int(-1)
        ));
        assert!(matches!(values.get(last).unwrap(), Value::Int(2)));
        assert_eq!(values.get(first).unwrap_err(), OwnershipError::StaleHandle);
        assert_eq!(values.insert(Value::Null), Err(OwnershipError::IdentityExhausted));
        assert_eq!(values.slots.len(), 1);
        values.leave_frame(frame).unwrap();
        assert!(values.lookup.is_empty());
    }

    #[test]
    fn loads_and_overwrites_reuse_storage_in_a_long_lived_frame() {
        let mut values = NativeOwnedValues::default();
        let frame = values.enter_frame().unwrap();
        let text = Arc::new("payload".to_string());
        let mut current = values.insert(Value::Str(text.clone())).unwrap();
        for _ in 0..100_000 {
            let next = values.duplicate(current).unwrap();
            values.release(current).unwrap();
            current = next;
        }
        assert_eq!(values.slots.len(), 2);
        assert!(values.lookup.capacity() <= 7);
        assert_eq!(Arc::strong_count(&text), 2);
        values.leave_frame(frame).unwrap();
        assert_eq!(Arc::strong_count(&text), 1);
        assert!(values.lookup.is_empty());
    }

    #[test]
    fn callback_frames_cannot_release_callers_and_promote_returns_without_copying() {
        let mut values = NativeOwnedValues::default();
        let outer = values.enter_frame().unwrap();
        let array = Arc::new(vec![Value::Int(7)]);
        let argument = values.insert(Value::Array(array.clone())).unwrap();
        let inner = values.enter_frame().unwrap();
        assert_eq!(values.release(argument), Err(OwnershipError::NotCurrentFrame));
        assert_eq!(values.leave_frame(outer), Err(OwnershipError::FrameOrder));
        let result = values.duplicate(argument).unwrap();
        let scratch = values.insert(Value::Str(Arc::new("temporary".into()))).unwrap();
        let owners = Arc::strong_count(&array);
        values.promote(result).unwrap();
        assert_eq!(Arc::strong_count(&array), owners);
        values.leave_frame(inner).unwrap();
        assert_eq!(values.get(scratch).unwrap_err(), OwnershipError::StaleHandle);
        assert!(
            matches!(values.get(result).unwrap(), Value::Array(value) if Arc::ptr_eq(value, &array))
        );
        values.release(argument).unwrap();
        values.leave_frame(outer).unwrap();
        assert_eq!(Arc::strong_count(&array), 1);
    }

    #[test]
    fn root_transfer_preserves_copy_on_write_snapshots() {
        let mut values = NativeOwnedValues::default();
        let frame = values.enter_frame().unwrap();
        let original = values.insert(Value::Array(Arc::new(vec![Value::Int(1)]))).unwrap();
        let copy = values.duplicate(original).unwrap();
        let Value::Array(mut changed) = values.take(copy).unwrap() else { panic!() };
        Arc::make_mut(&mut changed)[0] = Value::Int(2);
        assert!(
            matches!(values.get(original).unwrap(), Value::Array(array) if matches!(array[0], Value::Int(1)))
        );
        assert_eq!(values.take(copy).unwrap_err(), OwnershipError::StaleHandle);
        values.leave_frame(frame).unwrap();
    }

    #[test]
    fn repeated_error_cleanup_drops_only_live_roots_and_reuses_frames() {
        let mut values = NativeOwnedValues::default();
        let text = Arc::new("owned".to_string());
        let mut stale_frame = None;
        for _ in 0..10_000 {
            let frame = values.enter_frame().unwrap();
            if let Some(stale) = stale_frame {
                assert_eq!(values.leave_frame(stale), Err(OwnershipError::FrameOrder));
            }
            let first = values.insert(Value::Str(text.clone())).unwrap();
            let middle = values.insert(Value::Str(text.clone())).unwrap();
            let last = values.insert(Value::Str(text.clone())).unwrap();
            values.release(middle).unwrap();
            values.release(first).unwrap();
            // Simulate error exit with the remaining operand still live.
            values.leave_frame(frame).unwrap();
            assert_eq!(values.get(last).unwrap_err(), OwnershipError::StaleHandle);
            assert_eq!(Arc::strong_count(&text), 1);
            stale_frame = Some(frame);
        }
        assert_eq!(values.slots.len(), 3);
        assert!(values.frames.capacity() <= 4);
        assert!(values.lookup.is_empty());
    }

    #[test]
    fn store_drop_releases_unfinished_frames_and_identity_errors_are_atomic() {
        let text = Arc::new("owned".to_string());
        {
            let mut values = NativeOwnedValues::default();
            assert_eq!(values.insert(Value::Null), Err(OwnershipError::NoFrame));
            let frame = values.enter_frame().unwrap();
            let handle = values.insert(Value::Str(text.clone())).unwrap();
            assert_eq!(values.promote(handle), Err(OwnershipError::NoParentFrame));
            values.next_frame = None;
            assert_eq!(values.enter_frame(), Err(OwnershipError::IdentityExhausted));
            assert_eq!(values.frames.len(), 1);
            assert_eq!(values.frames[frame.depth].head, Some(0));
            assert_eq!(Arc::strong_count(&text), 2);
        }
        assert_eq!(Arc::strong_count(&text), 1);
    }

    #[test]
    fn mixed_nested_operations_match_an_independent_root_model() {
        let mut values = NativeOwnedValues::default();
        let mut frames = vec![values.enter_frame().unwrap()];
        let mut model = std::collections::BTreeMap::<u64, (usize, i64)>::new();
        let mut seed = 0x1234_5678_9abc_def0_u64;
        let mut peak = 0;
        for step in 0..25_000 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let depth = frames.len() - 1;
            let selected = if model.is_empty() {
                None
            } else {
                model
                    .iter()
                    .nth(seed as usize % model.len())
                    .map(|(&id, &entry)| (Handle(id), entry))
            };
            match (seed % 8, selected) {
                (0, _) | (_, None) => {
                    let handle = values.insert(Value::Int(step)).unwrap();
                    model.insert(handle.0, (depth, step));
                }
                (1, Some((source, (_, number)))) => {
                    let handle = values.duplicate(source).unwrap();
                    model.insert(handle.0, (depth, number));
                }
                (2, Some((handle, (owner, _)))) => {
                    if owner == depth {
                        values.release(handle).unwrap();
                        model.remove(&handle.0);
                        assert_eq!(values.get(handle).unwrap_err(), OwnershipError::StaleHandle);
                    } else {
                        assert_eq!(values.release(handle), Err(OwnershipError::NotCurrentFrame));
                    }
                }
                (3, Some((handle, (owner, number)))) if depth > 0 && owner == depth => {
                    values.promote(handle).unwrap();
                    model.insert(handle.0, (depth - 1, number));
                }
                (4, _) if frames.len() < 8 => frames.push(values.enter_frame().unwrap()),
                (5, _) if depth > 0 => {
                    values.leave_frame(frames.pop().unwrap()).unwrap();
                    model.retain(|_, (owner, _)| *owner != depth);
                }
                (6, Some((handle, (_, expected)))) => {
                    assert!(
                        matches!(values.get(handle).unwrap(), Value::Int(number) if *number == expected)
                    );
                }
                (7, Some((handle, (owner, expected)))) if owner == depth => {
                    let value = values.take(handle).unwrap();
                    assert!(matches!(value, Value::Int(number) if number == expected));
                    model.remove(&handle.0);
                    let replacement = values.insert(value).unwrap();
                    model.insert(replacement.0, (depth, expected));
                }
                _ => {}
            }
            peak = peak.max(model.len());
            assert_eq!(values.lookup.len(), model.len());
            assert!(values.slots.len() <= peak);
            for (&id, &(owner, number)) in &model {
                let index = values.index(Handle(id)).unwrap();
                assert_eq!(values.slots[index].owner, owner);
                assert!(
                    matches!(values.get(Handle(id)).unwrap(), Value::Int(actual) if *actual == number)
                );
            }
        }
        while let Some(frame) = frames.pop() {
            values.leave_frame(frame).unwrap();
        }
        assert!(values.lookup.is_empty());
        assert!(values.slots.iter().all(|slot| slot.value.is_none()));
    }
}
