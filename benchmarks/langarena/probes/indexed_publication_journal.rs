// Exploratory storage model using real Kujo Values, not a runtime implementation.
// Counts requested allocation traffic, including publication where specified.
use kujo::interpreter::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
static TRACK: AtomicBool = AtomicBool::new(false);
static REQUESTS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
struct Counter;
fn record(n: usize) {
    if TRACK.load(Relaxed) {
        REQUESTS.fetch_add(1, Relaxed);
        BYTES.fetch_add(n as u64, Relaxed);
    }
}
// SAFETY: System receives unchanged allocation/deallocation arguments.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        record(l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        record(l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        record(n);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn begin() {
    REQUESTS.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    TRACK.store(true, Relaxed);
}
fn end() -> (u64, u64) {
    TRACK.store(false, Relaxed);
    (REQUESTS.load(Relaxed), BYTES.load(Relaxed))
}
fn integer(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        _ => panic!("unexpected value"),
    }
}
// Numeric-array feasibility model only: no interpreter integration or speed claim.
// Both modes keep a real, unchanged public Arc<Vec<Value>> until publication.
// Bounded scalar storage; non-scalar writes materialize the ordinary array.
// This is still a standalone model, not a new public/runtime Value variant.
enum Scalar {
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Str(Arc<String>),
}
impl Scalar {
    fn from_value(value: Value) -> Result<Self, Value> {
        Ok(match value {
            Value::Int(v) => Self::Int(v),
            Value::Float(v) => Self::Float(v),
            Value::Bool(v) => Self::Bool(v),
            Value::Null => Self::Null,
            Value::Str(v) => Self::Str(v),
            other => return Err(other),
        })
    }
    fn value(&self) -> Value {
        match self {
            Self::Int(v) => Value::Int(*v),
            Self::Float(v) => Value::Float(*v),
            Self::Bool(v) => Value::Bool(*v),
            Self::Null => Value::Null,
            Self::Str(v) => Value::Str(v.clone()),
        }
    }
    fn into_value(self) -> Value {
        match self {
            Self::Int(v) => Value::Int(v),
            Self::Float(v) => Value::Float(v),
            Self::Bool(v) => Value::Bool(v),
            Self::Null => Value::Null,
            Self::Str(v) => Value::Str(v),
        }
    }
}
enum Active {
    Flat(Arc<Vec<Value>>),
    Journal { base: Arc<Vec<Value>>, writes: Vec<(usize, Value)> },
    Inline { base: Arc<Vec<Value>>, writes: [Option<(usize, Scalar)>; 4] },
}
impl Active {
    fn new(owner: &Arc<Vec<Value>>, mode: &str) -> Self {
        if mode == "flat" || Arc::strong_count(owner) > 1 {
            return Self::Flat(owner.clone());
        }
        if mode == "inline" {
            Self::Inline { base: owner.clone(), writes: std::array::from_fn(|_| None) }
        } else {
            Self::Journal { base: owner.clone(), writes: Vec::new() }
        }
    }
    fn get(&self, index: usize) -> Value {
        match self {
            Self::Flat(values) => values[index].clone(),
            Self::Journal { base, writes } => writes
                .iter()
                .rev()
                .find(|(i, _)| *i == index)
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| base[index].clone()),
            Self::Inline { base, writes } => writes
                .iter()
                .flatten()
                .find(|(i, _)| *i == index)
                .map(|(_, value)| value.value())
                .unwrap_or_else(|| base[index].clone()),
        }
    }
    fn set(&mut self, index: usize, value: Value) {
        match self {
            Self::Flat(values) => Arc::make_mut(values)[index] = value,
            Self::Journal { base, writes } => {
                assert!(index < base.len());
                if let Some((_, slot)) = writes.iter_mut().find(|(i, _)| *i == index) {
                    *slot = value;
                } else {
                    writes.push((index, value));
                }
            }
            Self::Inline { base, writes } => {
                assert!(index < base.len());
                let scalar = match Scalar::from_value(value) {
                    Ok(value) => value,
                    Err(value) => {
                        self.materialize();
                        self.set(index, value);
                        return;
                    }
                };
                if let Some((_, slot)) = writes.iter_mut().flatten().find(|(i, _)| *i == index) {
                    *slot = scalar;
                } else if let Some(slot) = writes.iter_mut().find(|v| v.is_none()) {
                    *slot = Some((index, scalar));
                } else {
                    self.materialize();
                    self.set(index, scalar.into_value());
                }
            }
        }
    }
    fn materialize(&mut self) {
        let flat = match self {
            Self::Flat(_) => return,
            Self::Journal { base, writes } => {
                let mut flat = base.clone();
                for (index, value) in writes.drain(..) {
                    Arc::make_mut(&mut flat)[index] = value;
                }
                flat
            }
            Self::Inline { base, writes } => {
                let mut flat = base.clone();
                for slot in writes.iter_mut() {
                    if let Some((index, value)) = slot.take() {
                        Arc::make_mut(&mut flat)[index] = value.into_value();
                    }
                }
                flat
            }
        };
        *self = Self::Flat(flat);
    }
    fn snapshot(&mut self) -> Arc<Vec<Value>> {
        self.materialize();
        let Self::Flat(values) = self else { unreachable!() };
        values.clone()
    }
    fn publish(self, owner: &mut Arc<Vec<Value>>) {
        match self {
            Self::Flat(values) => *owner = values,
            Self::Journal { mut base, writes } => {
                if writes.is_empty() {
                    return;
                }
                if Arc::ptr_eq(&base, owner) {
                    drop(base);
                    let values = Arc::make_mut(owner);
                    for (index, value) in writes {
                        values[index] = value;
                    }
                } else {
                    let values = Arc::make_mut(&mut base);
                    for (index, value) in writes {
                        values[index] = value;
                    }
                    *owner = base;
                }
            }
            Self::Inline { mut base, writes } => {
                if writes.iter().all(Option::is_none) {
                    return;
                }
                if Arc::ptr_eq(&base, owner) {
                    drop(base);
                    let values = Arc::make_mut(owner);
                    for (index, value) in writes.into_iter().flatten() {
                        values[index] = value.into_value();
                    }
                } else {
                    let values = Arc::make_mut(&mut base);
                    for (index, value) in writes.into_iter().flatten() {
                        values[index] = value.into_value();
                    }
                    *owner = base;
                }
            }
        }
    }
}
fn run(len: usize, updates: usize, policy: &str, pattern: &str, mode: &str) {
    let initial: Arc<Vec<Value>> = Arc::new((0..len).map(|i| Value::Int(i as i64)).collect());
    let mut owner = initial.clone();
    let mut model: Vec<i64> = (0..len).map(|i| i as i64).collect();
    let mut totals = (0, 0);
    let calls = 16;
    for call in 0..calls {
        let before = model.clone();
        let mut escaped_model = model.clone();
        let fresh = (policy == "fresh").then(|| owner.clone());
        let replacement = (policy == "replace").then(|| Arc::new(vec![Value::Int(-1); len]));
        let mut observed = None;
        let mut escaped = None;
        begin();
        let mut active = Active::new(&owner, mode);
        let completed = if policy == "partial_error" { updates.div_ceil(2) } else { updates };
        for step in 0..completed {
            let offset = match pattern {
                "same" => 0,
                "two" => step % 2,
                _ => step * 31,
            };
            let index = (call * 17 + offset) % len;
            let next = integer(&active.get(index)) + 1;
            active.set(index, Value::Int(next));
            model[index] += 1;
            assert_eq!(integer(&active.get(index)), model[index]);
            if step == completed / 2 {
                if policy == "during" {
                    observed = Some(owner.clone());
                }
                if policy == "escape" {
                    escaped = Some(active.snapshot());
                    escaped_model.copy_from_slice(&model);
                }
            }
        }
        // Public data is a real readable flat array, not a marker or locked view.
        for i in 0..len {
            assert_eq!(integer(&owner[i]), before[i]);
            assert_eq!(integer(&active.get(i)), model[i]);
        }
        if let Some(replacement) = replacement {
            owner = replacement;
        }
        // The partial-error policy publishes completed writes just like return.
        active.publish(&mut owner);
        let measured = end();
        totals.0 += measured.0;
        totals.1 += measured.1;
        for i in 0..len {
            assert_eq!(integer(&owner[i]), model[i]);
            assert_eq!(integer(&initial[i]), i as i64);
            if let Some(values) = &fresh {
                assert_eq!(integer(&values[i]), before[i]);
            }
            if let Some(values) = &observed {
                assert_eq!(integer(&values[i]), before[i]);
            }
            if let Some(values) = &escaped {
                assert_eq!(integer(&values[i]), escaped_model[i]);
            }
        }
    }
    println!("{{\"mode\":\"{}\",\"length\":{},\"writes\":{},\"policy\":\"{}\",\"pattern\":\"{}\",\"calls\":{},\"requests\":{},\"bytes\":{},\"value_size\":{},\"active_size\":{},\"semantics_passed\":true}}", mode,len,updates,policy,pattern,calls,totals.0,totals.1,std::mem::size_of::<Value>(),std::mem::size_of::<Active>());
}
fn storage_controls() {
    assert!(
        std::mem::size_of::<Active>() <= std::mem::size_of::<Value>(),
        "model exceeds existing Value storage"
    );
    let mut owner = Arc::new(vec![Value::Int(0); 8]);
    let mut active = Active::new(&owner, "inline");
    assert!(matches!(active, Active::Inline { .. }));
    active.set(0, Value::Float(2.5));
    active.set(1, Value::Bool(true));
    active.set(2, Value::Str(Arc::new("kept".into())));
    active.set(3, Value::Null);
    assert!(matches!(active, Active::Inline { .. }));
    assert!(matches!(active.get(0),Value::Float(v) if v==2.5));
    assert!(matches!(active.get(1), Value::Bool(true)));
    assert!(matches!(active.get(2),Value::Str(v) if v.as_str()=="kept"));
    active.set(4, Value::Array(Arc::new(vec![Value::Int(9)])));
    assert!(matches!(active, Active::Flat(_)));
    assert!(owner.iter().all(|v| matches!(v, Value::Int(0))));
    let snapshot = active.snapshot();
    active.set(0, Value::Int(7));
    active.publish(&mut owner);
    assert!(matches!(snapshot[0],Value::Float(v) if v==2.5));
    assert!(matches!(owner[0], Value::Int(7)));
    assert!(matches!(&owner[4],Value::Array(v) if integer(&v[0])==9));
    let mut dense = Arc::new(vec![Value::Int(0); 8]);
    let mut active = Active::new(&dense, "inline");
    for i in 0..5 {
        active.set(i, Value::Int(i as i64 + 1));
    }
    assert!(matches!(active, Active::Flat(_)));
    active.publish(&mut dense);
    for i in 0..5 {
        assert_eq!(integer(&dense[i]), i as i64 + 1);
    }
    let pointer = Arc::as_ptr(&dense);
    Active::new(&dense, "inline").publish(&mut dense);
    assert_eq!(pointer, Arc::as_ptr(&dense));
    eprintln!("INLINE STORAGE CONTROLS PASS");
}
fn main() {
    storage_controls();
    for len in [128, 2048, 32768] {
        for writes in [1, 8, 128, 512] {
            for policy in ["once", "fresh", "during", "escape", "replace", "partial_error"] {
                for pattern in ["spread", "same", "two"] {
                    for mode in ["flat", "journal", "inline"] {
                        run(len, writes, policy, pattern, mode);
                    }
                }
            }
        }
    }
}
