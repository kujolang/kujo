//! Bounded, non-owning metadata for immutable string allocations. Arc::make_mut
//! dissociates weak references before mutation, so cached lengths cannot go stale.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::{Arc, Weak};

const CAPACITY: usize = 64;
#[derive(Default)]
struct Cache {
    entries: VecDeque<(Weak<String>, usize)>,
    #[cfg(test)]
    scans: usize,
}
impl Cache {
    fn len(&mut self, text: &Arc<String>) -> usize {
        if let Some(index) =
            self.entries.iter().position(|(key, _)| key.as_ptr() == Arc::as_ptr(text))
        {
            let entry = self.entries.remove(index).unwrap();
            let len = entry.1;
            self.entries.push_front(entry);
            return len;
        }
        let len = text.chars().count();
        #[cfg(test)]
        {
            self.scans += 1;
        }
        if self.entries.len() == CAPACITY {
            self.entries.pop_back();
        }
        self.entries.push_front((Arc::downgrade(text), len));
        len
    }
}
thread_local! { static CACHE: RefCell<Cache> = RefCell::new(Cache::default()); }

pub(crate) fn len(text: &Arc<String>) -> usize {
    // Short strings are cheaper to scan than to put into the metadata cache.
    if text.len() <= 32 {
        text.chars().count()
    } else {
        CACHE.with(|cache| cache.borrow_mut().len(text))
    }
}

pub(crate) fn char_at(text: &Arc<String>, index: i64) -> Option<char> {
    let count = len(text);
    let index = if index < 0 { (count as i64).checked_add(index)? } else { index };
    let index = usize::try_from(index).ok()?;
    if count == text.len() {
        text.as_bytes().get(index).copied().map(char::from)
    } else {
        text.chars().nth(index)
    }
}

pub(crate) fn substring(text: &Arc<String>, start: usize, end: usize) -> String {
    let count = len(text);
    let start = start.min(count);
    let end = end.min(count);
    if count == text.len() {
        text[start..end].to_string()
    } else {
        text.chars().skip(start).take(end.saturating_sub(start)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_allocations_scan_once_and_mutation_invalidates_without_retention() {
        let mut cache = Cache::default();
        let mut text = Arc::new("a".repeat(100));
        for _ in 0..100 {
            assert_eq!(cache.len(&text), 100);
        }
        assert_eq!(cache.scans, 1);
        let alias = text.clone();
        Arc::make_mut(&mut text).push('é');
        assert_eq!(cache.len(&text), 101);
        assert_eq!(cache.len(&alias), 100);
        drop(alias);
        Arc::make_mut(&mut text).push('🙂');
        assert_eq!(cache.len(&text), 102);
        assert_eq!(cache.scans, 3);
        drop(text);
        assert!(cache.entries.iter().all(|(key, _)| key.upgrade().is_none()));
        for _ in 0..100 {
            cache.len(&Arc::new("x".repeat(100)));
        }
        assert_eq!(cache.entries.len(), CAPACITY);
    }
}
