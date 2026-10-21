//! Immutable shared queue spines. Updating a queue never edits another value.
use super::Value;
use std::sync::{Arc, OnceLock};

type Link = Option<Arc<Node>>;
struct Node {
    value: Arc<Value>,
    value_depth: usize,
    next: Link,
    len: usize,
    max_depth: usize,
    reversed: OnceLock<Link>,
}
impl Node {
    fn prepend(value: Value, next: Link) -> Arc<Self> {
        let depth = Value::nesting_depth(&value);
        Self::prepend_shared(Arc::new(value), depth, next)
    }
    fn prepend_shared(value: Arc<Value>, depth: usize, next: Link) -> Arc<Self> {
        Arc::new(Self {
            len: 1 + next.as_ref().map_or(0, |n| n.len),
            max_depth: depth.max(next.as_ref().map_or(0, |n| n.max_depth)),
            value,
            value_depth: depth,
            next,
            reversed: OnceLock::new(),
        })
    }
    fn reversed(head: &Arc<Self>) -> Link {
        head.reversed
            .get_or_init(|| {
                let mut out = None;
                let mut cursor = Some(head.as_ref());
                while let Some(node) = cursor {
                    out =
                        Some(Self::prepend_shared(Arc::clone(&node.value), node.value_depth, out));
                    cursor = node.next.as_deref();
                }
                out
            })
            .clone()
    }
}
impl Drop for Node {
    fn drop(&mut self) {
        // Releasing a long unique spine must not recurse once per queue item.
        let mut next = self.next.take();
        while let Some(link) = next {
            match Arc::try_unwrap(link) {
                Ok(mut node) => next = node.next.take(),
                Err(_) => break,
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct PersistentQueue {
    front: Link,
    rear: Link,
}
impl PersistentQueue {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.front.as_ref().map_or(0, |n| n.len) + self.rear.as_ref().map_or(0, |n| n.len)
    }
    pub fn is_empty(&self) -> bool {
        self.front.is_none()
    }
    pub fn front(&self) -> Option<&Value> {
        self.front.as_ref().map(|n| n.value.as_ref())
    }
    pub fn push_back(&mut self, value: Value) {
        if self.front.is_none() {
            self.front = Some(Node::prepend(value, None));
        } else {
            self.rear = Some(Node::prepend(value, self.rear.take()));
        }
    }
    pub fn pop_front(&mut self) -> Option<Value> {
        let head = self.front.take()?;
        let value = head.value.as_ref().clone();
        self.front = head.next.clone();
        if self.front.is_none() {
            self.front = self.rear.take().as_ref().and_then(Node::reversed);
        }
        Some(value)
    }
    pub(crate) fn nesting_depth(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            1 + self
                .front
                .as_ref()
                .map_or(0, |n| n.max_depth)
                .max(self.rear.as_ref().map_or(0, |n| n.max_depth))
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &Value> {
        let forward = std::iter::successors(self.front.as_deref(), |n| n.next.as_deref())
            .map(|n| n.value.as_ref());
        let backward: Vec<_> = std::iter::successors(self.rear.as_deref(), |n| n.next.as_deref())
            .map(|n| n.value.as_ref())
            .collect();
        forward.chain(backward.into_iter().rev())
    }
}
impl FromIterator<Value> for PersistentQueue {
    fn from_iter<T: IntoIterator<Item = Value>>(items: T) -> Self {
        let mut queue = Self::new();
        for item in items {
            queue.push_back(item);
        }
        queue
    }
}
impl From<std::collections::VecDeque<Value>> for PersistentQueue {
    fn from(items: std::collections::VecDeque<Value>) -> Self {
        items.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clones_and_updates_share_spines_and_cached_reversals() {
        let q: PersistentQueue = (0..10_000).map(Value::Int).collect();
        let mut a = q.clone();
        assert!(Arc::ptr_eq(q.front.as_ref().unwrap(), a.front.as_ref().unwrap()));
        assert!(Arc::ptr_eq(q.rear.as_ref().unwrap(), a.rear.as_ref().unwrap()));
        a.push_back(Value::Int(10_000));
        assert!(Arc::ptr_eq(
            q.rear.as_ref().unwrap(),
            a.rear.as_ref().unwrap().next.as_ref().unwrap()
        ));
        let mut b = q.clone();
        let mut c = q.clone();
        assert!(matches!(b.pop_front(), Some(Value::Int(0))));
        c.pop_front();
        assert!(Arc::ptr_eq(b.front.as_ref().unwrap(), c.front.as_ref().unwrap()));
        assert_eq!(q.len(), 10_000);
        assert_eq!(a.len(), 10_001);
        assert_eq!(b.nesting_depth(), 1);
        let mut visits = 0;
        assert_eq!(Value::walk_nesting_depth(&Value::Queue(q.clone()), || visits += 1), 1);
        assert_eq!(visits, 1, "queue validation scanned its elements");
        for i in 1..10_000 {
            assert!(matches!(b.pop_front(), Some(Value::Int(n)) if n == i));
        }
        assert!(b.pop_front().is_none());
    }
    #[test]
    fn long_spines_drop_without_recursive_stack_growth() {
        let q: PersistentQueue = (0..100_000).map(Value::Int).collect();
        drop(q);
    }
    #[test]
    fn depth_metadata_tracks_removal_and_preserves_snapshots() {
        let mut q = PersistentQueue::new();
        q.push_back(Value::array(vec![Value::array(vec![Value::Int(0)])]));
        q.push_back(Value::Int(2));
        let copy = q.clone();
        assert_eq!(q.nesting_depth(), 3);
        q.pop_front();
        assert_eq!(q.nesting_depth(), 1);
        assert_eq!(copy.nesting_depth(), 3);
        q.pop_front();
        assert_eq!(q.nesting_depth(), 0);
    }
}

impl From<Vec<Value>> for PersistentQueue {
    fn from(items: Vec<Value>) -> Self {
        items.into_iter().collect()
    }
}
