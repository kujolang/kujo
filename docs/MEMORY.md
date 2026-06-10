# Kujo Language - Memory Model

This document describes Kujo's memory management, ownership model, and garbage collection strategy.

**Last Updated**: January 27, 2026  
**Version**: v0.9.0

---

## Table of Contents

1. [Overview](#overview)
2. [Value Ownership Model](#value-ownership-model)
3. [Environment Lifetime Management](#environment-lifetime-management)
4. [Closure Capture Semantics](#closure-capture-semantics)
5. [Garbage Collection Strategy](#garbage-collection-strategy)
6. [Memory Safety Guarantees](#memory-safety-guarantees)
7. [LeakyFunctionBody Issue](#leakyfunctionbody-issue)
8. [Memory Patterns](#memory-patterns)
9. [Performance Characteristics](#performance-characteristics)
10. [Best Practices](#best-practices)

---

## Overview

Kujo uses **Rust's ownership system** for memory safety, combined with **reference counting** for shared ownership and **mutex-based locking** for thread safety.

### Memory Management Strategy

| Type | Ownership | Thread-Safety | Cleanup |
|------|-----------|---------------|---------|
| Primitives (Int, Float, Bool) | Copy | Yes | Stack |
| Strings, Arrays, Dicts | Clone | Single-thread | Arc<T> for sharing |
| Functions (closures) | Arc<Mutex<Environment>> | Yes | Reference counting |
| Channels | Arc<Mutex<...>> | Yes | Reference counting |
| Promises | Arc<Mutex<...>> | Yes | Reference counting |

**Key Principle**: All shared mutable state uses `Arc<Mutex<T>>` for safe concurrent access.

---

## Value Ownership Model

### Value Type Definition

```rust
// src/interpreter/value.rs:243-372
#[derive(Clone)]
pub enum Value {
    // Primitives (stack-allocated, cheap to copy)
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    
    // Heap-allocated collections (cloned when needed)
    Str(String),
    Array(Vec<Value>),
    Dict(HashMap<String, Value>),
    
    // Thread-safe shared types
    Channel(Arc<Mutex<(Sender<Value>, Receiver<Value>)>>),
    Promise { 
        receiver: Arc<Mutex<Receiver<Result<Value, String>>>>,
        // ...
    },
    
    // Functions with optional closure environment
    Function(
        Vec<String>,                      // Parameters
        LeakyFunctionBody,                // Body statements
        Option<Arc<Mutex<Environment>>>,  // Captured environment
    ),
    
    // ...
}
```

### Ownership Rules

1. **Values are cloned when stored**:
   ```kujo
   x := [1, 2, 3]
   y := x  # Array is cloned, not moved
   x.push(4)  # Both x and y now [1, 2, 3, 4]? No - independent copies!
   ```

2. **Values in collections are cloned**:
   ```kujo
   arr := [x, y, z]  # Each value is cloned into array
   ```

3. **Function parameters are cloned**:
   ```kujo
   func process(data) {
       data.push(10)  # Modifies local copy, not original
   }
   
   arr := [1, 2, 3]
   process(arr)
   print(arr)  # Still [1, 2, 3]
   ```

### Value Cloning Cost

| Value Type | Clone Cost | Notes |
|------------|------------|-------|
| Int, Float, Bool | O(1) | Stack copy |
| Null | O(1) | Zero-size type |
| Str | O(n) | Allocates new string |
| Array | O(n*m) | Deep clone of all elements |
| Dict | O(n*m) | Deep clone of all entries |
| Function | O(1) | Arc clone (ref count increment) |
| Channel | O(1) | Arc clone (ref count increment) |

**Note**: "Deep clone" means recursively cloning nested structures.

---

## Environment Lifetime Management

The `Environment` struct manages variable scopes using a **stack of hash maps**.

### Environment Structure

```rust
// src/interpreter/environment.rs:28-31
#[derive(Clone, Debug)]
pub struct Environment {
    pub scopes: Vec<HashMap<String, Value>>,
}
```

### Scope Stack

```
┌───────────────────────────────────┐
│ Function Scope (innermost)        │  ◄── scopes[2]
├───────────────────────────────────┤
│ Block Scope                       │  ◄── scopes[1]
├───────────────────────────────────┤
│ Global Scope (outermost)          │  ◄── scopes[0]
└───────────────────────────────────┘
```

### Variable Lookup

**Algorithm** (`src/interpreter/environment.rs:53-61`):
```rust
pub fn get(&self, name: &str) -> Option<Value> {
    // Search from innermost (end) to outermost (start)
    for scope in self.scopes.iter().rev() {
        if let Some(value) = scope.get(name) {
            return Some(value.clone());  // Clone on read
        }
    }
    None  // Variable not found
}
```

**Example**:
```kujo
x := 10      # Global scope: scopes[0]["x"] = 10

if true {
    scoped_x := 20  # Block scope variable
    print(scoped_x) # Prints 20
}

print(x)     # Prints 10 (scopes[1] popped, finds in scopes[0])
```

### Scope Lifecycle

**Function Call**:
```rust
fn call_function(&mut self, func: Value, args: Vec<Value>) {
    // 1. Push new scope
    self.env.push_scope();
    
    // 2. Bind parameters
    for (param, arg) in params.iter().zip(args.iter()) {
        self.env.define(param.clone(), arg.clone());
    }
    
    // 3. Execute function body
    self.eval_stmts(&body);
    
    // 4. Pop scope (local variables destroyed)
    self.env.pop_scope();
}
```

**Lifetime**:
```
┌────────────────────┐
│  Before Call       │  scopes: [global]
└────────┬───────────┘
         │ push_scope()
         ▼
┌────────────────────┐
│  During Call       │  scopes: [global, function]
└────────┬───────────┘
         │ pop_scope()
         ▼
┌────────────────────┐
│  After Call        │  scopes: [global]
└────────────────────┘
```

---

## Closure Capture Semantics

Closures **capture the environment** at definition time, allowing access to outer variables.

### Closure Creation

**Kujo Code**:
```kujo
func make_counter() {
    count := 0
    
    # This closure captures 'count' from outer scope
    func increment() {
        count := count + 1
        return count
    }
    
    return increment
}

counter := make_counter()
print(counter())  # 1
print(counter())  # 2
print(counter())  # 3
```

**Implementation** (`src/interpreter/mod.rs:2493-2506`):
```rust
Expr::Function { params, body, .. } => {
    // Anonymous function (closure) captures current environment
    Value::Function(
        params.clone(),
        LeakyFunctionBody::new(body.clone()),
        Some(Arc::new(Mutex::new(self.env.clone()))),  // Captured!
    )
}
```

### Closure Invocation

**Process** (`src/interpreter/mod.rs:1048-1101`):
```rust
Value::Function(params, body, captured_env) => {
    if let Some(closure_env_ref) = captured_env {
        // 1. Save current environment
        let saved_env = self.env.clone();
        
        // 2. Switch to captured environment
        self.env = closure_env_ref.lock().unwrap().clone();
        self.env.push_scope();  // New scope for parameters
        
        // 3. Bind parameters
        for (param, arg) in params.iter().zip(args.iter()) {
            self.env.define(param.clone(), arg.clone());
        }
        
        // 4. Execute function body
        self.eval_stmts(body.get());
        
        // 5. Pop parameter scope
        self.env.pop_scope();
        
        // 6. Write back modified environment
        *closure_env_ref.lock().unwrap() = self.env.clone();
        
        // 7. Restore original environment
        self.env = saved_env;
    }
}
```

### Closure Memory Diagram

```
┌─────────────────────────────────────────────────┐
│ make_counter()                                  │
│                                                 │
│  ┌──────────────────────────┐                  │
│  │ Environment              │                  │
│  │ ┌──────────────────────┐ │                  │
│  │ │ count = 0            │ │                  │
│  │ └──────────────────────┘ │                  │
│  └────────────┬─────────────┘                  │
│               │                                 │
│               │ Arc<Mutex<Environment>>         │
│               │ (shared ownership)              │
│               │                                 │
│  ┌────────────▼─────────────┐                  │
│  │ increment (closure)      │                  │
│  │  - params: []            │                  │
│  │  - body: [count+=1; ...]  │                  │
│  │  - captured_env ───────┐ │                  │
│  └────────────────────────┘ │                  │
│                             │                  │
│                             │ References       │
│                             │ outer env        │
│                             └──────────────────┤
└─────────────────────────────────────────────────┘

After make_counter() returns:
- increment closure survives (Arc keeps Environment alive)
- count variable persists in closure's Environment
- Each call to increment() modifies the same count
```

### Capture Modes

**Current Implementation**: All closures capture **by reference** (via `Arc<Mutex<Environment>>`).

**Named Functions** (do not capture):
```kujo
func outer() {
    x := 10
    
    func inner() {
        print(x)  # Error: x not defined
    }
    
    inner()
}
```

**Anonymous Functions** (capture environment):
```kujo
func outer() {
    x := 10
    
    func inner() {
        print(x)  # OK: closure captures x
    }
    
    inner()  # Prints: 10
}
```

---

## Garbage Collection Strategy

Kujo uses **reference counting** (Arc) combined with **scope-based destruction**.

### Reference Counting with Arc

**How it works**:
1. `Arc::new(value)` creates ref count = 1
2. `arc.clone()` increments ref count
3. Dropping `Arc` decrements ref count
4. When ref count reaches 0, value is freed

**Example**:
```rust
let env = Arc::new(Mutex::new(Environment::new()));  // ref count = 1

let closure1 = Value::Function(
    params,
    body,
    Some(env.clone()),  // ref count = 2
);

let closure2 = closure1.clone();  // ref count = 3

drop(closure1);  // ref count = 2
drop(closure2);  // ref count = 1
drop(env);       // ref count = 0 → Environment freed
```

### Value Destruction

**Automatic**: Values are destroyed when:
1. Variable goes out of scope
2. Last reference to Arc is dropped
3. Collection containing value is destroyed

**Example**:
```kujo
x := [1, 2, 3]      # Array allocated
y := {"key": "val"} # Dict allocated
print(len(x))
print(y["key"])
# x and y are reclaimed when script execution ends
```

### Cycle Detection

**Problem**: Reference cycles prevent ref count from reaching zero.

**Example**:
```kujo
# Potential cycle (not possible in current Kujo, but illustrative)
a := {}
b := {}
a["ref"] := b
b["ref"] := a

# a and b reference each other → ref counts never reach 0
```

**Current Status**: Kujo doesn't have explicit cycle detection. Most patterns naturally avoid cycles, but long-running programs with complex data structures could leak memory.

**Future Work** (Roadmap Task #29): Implement cycle detection or switch to tracing GC.

---

## LeakyFunctionBody Issue

### The Problem

**Root Cause**: Deep recursion in `Drop` trait implementation.

```rust
// Function bodies are Vec<Stmt>
pub struct FunctionBody(Vec<Stmt>);

// Stmt contains nested Vec<Stmt>
pub enum Stmt {
    If { condition: Expr, then_branch: Vec<Stmt>, else_branch: Vec<Stmt> },
    While { condition: Expr, body: Vec<Stmt> },
    For { ... body: Vec<Stmt> },
    // ... many more with nested Vec<Stmt>
}
```

**What happens**:
```
drop(FunctionBody)
  └─> drop(Vec<Stmt>)
       └─> drop(Stmt::If)
            └─> drop(then_branch: Vec<Stmt>)
                 └─> drop(Stmt::While)
                      └─> drop(body: Vec<Stmt>)
                           └─> ... (deep recursion)
```

**Result**: Stack overflow during program shutdown for deeply nested code.

### Current Workaround

**LeakyFunctionBody** (`src/interpreter/value.rs:20-42`):
```rust
#[derive(Clone)]
pub struct LeakyFunctionBody(ManuallyDrop<Arc<Vec<Stmt>>>);

impl LeakyFunctionBody {
    pub fn new(body: Vec<Stmt>) -> Self {
        LeakyFunctionBody(ManuallyDrop::new(Arc::new(body)))
    }

    pub fn get(&self) -> &Vec<Stmt> {
        &self.0
    }
}

// No Drop implementation → memory leaked
// OS reclaims all memory at program shutdown anyway
```

**Trade-off**:
- ❌ Memory leaked during execution (functions never freed)
- ✅ No stack overflow
- ✅ OS cleans up at shutdown
- ✅ Acceptable for short-lived programs

### Future Solutions

**1. Iterative Drop** (Recommended):
```rust
impl Drop for FunctionBody {
    fn drop(&mut self) {
        let mut stack = vec![self.0.clone()];
        
        while let Some(node) = stack.pop() {
            // Extract child nodes, push to stack
            // Avoid recursive drop calls
        }
    }
}
```

**2. Arena Allocation**:
```rust
use typed_arena::Arena;

struct Interpreter {
    arena: Arena<Stmt>,  // Allocate all Stmts here
}

// Drop entire arena at once (no recursion)
```

**3. Flatten AST**:
```rust
// Instead of nested Vec<Stmt>
pub struct Stmt {
    kind: StmtKind,
    children_indices: Vec<usize>,  // Indices into flat Vec
}
```

**Roadmap**: Task #29 - Fix LeakyFunctionBody with iterative drop.

---

## Memory Safety Guarantees

Kujo inherits Rust's memory safety guarantees:

### No Unsafe Memory Access

✅ **Guaranteed**:
- No null pointer dereferences (Option<T> instead of null)
- No use-after-free (ownership prevents it)
- No data races (Arc<Mutex<T>> enforces locking)
- No buffer overflows (bounds-checked arrays)

❌ **Not Guaranteed** (language-level):
- Memory leaks (LeakyFunctionBody issue)
- Reference cycles (no cycle detection yet)
- Infinite loops (halting problem)

### Thread Safety

All shared mutable state uses `Mutex` for exclusive access:

```rust
// Channel: only one thread can send/receive at a time
Arc<Mutex<(Sender<Value>, Receiver<Value>)>>

// Promise: only one thread can poll at a time
Arc<Mutex<Receiver<Result<Value, String>>>>

// Closure environment: synchronized access
Arc<Mutex<Environment>>
```

**Rust's type system prevents**:
- Data races at compile time
- Sending non-thread-safe types across threads
- Aliasing mutable references

---

## Memory Patterns

### Pattern 1: Temporary Scope

**Use Case**: Limit variable lifetime.

```kujo
large_data := read_file("huge.csv")  # 1 GB allocated
process(large_data)

next_operation()  # Memory available
```

### Pattern 2: Explicit Clearing

**Use Case**: Free memory in long-running loop.

```kujo
for i in range(1000) {
    data := expensive_computation()
    use(data)
    
    released := null  # Release reference (value destroyed)
    use(released)
}
```

### Pattern 3: Avoid Cloning Large Structures

**Bad** (many clones):
```kujo
arr := range(1000000)  # 1M element array

func process(data) {
    # ... work with data
}

for i in range(100) {
    process(arr)  # Clones 1M elements each call!
}
```

**Good** (pass indices):
```kujo
arr := range(1000000)

func process(arr, start, end) {
    for i in range(start, end) {
        print(arr[i])  # Work with arr[i]
    }
}

for chunk in range(0, 100) {
    process(arr, chunk * 10000, (chunk + 1) * 10000)  # No clone!
}
```

### Pattern 4: Reuse Allocations

**Bad** (many allocations):
```kujo
for i in range(1000) {
    result := []  # New array each iteration
    for j in range(100) {
        result.push(compute(i, j))
    }
    process(result)
}
```

**Good** (reuse array):
```kujo
result := []
for i in range(1000) {
    result.clear()  # Keep allocation, reset length
    for j in range(100) {
        result.push(compute(i, j))
    }
    process(result)
}
```

---

## Performance Characteristics

### Value Sizes

| Type | Stack Size | Heap Size | Clone Cost |
|------|------------|-----------|------------|
| Int | 8 bytes | 0 | O(1) |
| Float | 8 bytes | 0 | O(1) |
| Bool | 1 byte | 0 | O(1) |
| Str | 24 bytes | n bytes | O(n) |
| Array(n) | 24 bytes | n * sizeof(Value) | O(n*m) |
| Dict(n) | 48 bytes | n * (key + value) | O(n*m) |
| Function | 64 bytes | body size | O(1) (Arc) |
| Channel | 16 bytes | small | O(1) (Arc) |

**Note**: `sizeof(Value)` ≈ 64-80 bytes (large enum).

### Memory Overhead

**Per-Value Overhead**: ~64 bytes (Value enum tag + largest variant)

**Example**:
```kujo
arr := [1, 2, 3]
# Memory: 24 (Vec) + 3 * 64 (Values) = ~216 bytes
# Actual data: 3 * 8 = 24 bytes integers
# Overhead: 192 bytes (8x)!
```

**Optimization Ideas** (future):
- Specialized `IntArray` for int-only arrays
- `FloatArray` for float-only arrays
- Smaller Value enum (box large variants)

### Allocation Patterns

**Stack vs Heap**:
- Primitives: Stack (no heap allocation)
- Collections: Heap (Vec, HashMap allocate on heap)
- Functions: Heap (LeakyFunctionBody wraps Arc<Vec<Stmt>>)

**Allocation Frequency**:
```kujo
for i in range(1000000) {
    x := i  # Stack-only, no allocation
}

for i in range(1000000) {
    arr := [i]  # 1M heap allocations!
}
```

---

## Best Practices

### 1. Minimize Cloning

**Bad**:
```kujo
func process_many(items) {
    for item in items {  # Each iteration clones item!
        work_with(item)
    }
}

large_array := range(10000)
process_many(large_array)  # Clones entire array + each element
```

**Good**:
```kujo
# Process in-place or use indices
func process_many(items) {
    for i in range(len(items)) {
        work_with(items[i])  # No clone (if items not cloned)
    }
}
```

### 2. Reuse Allocations

```kujo
# Bad
for i in range(1000) {
    result := do_work(i)
    results.push(result)
}

# Good: Pre-allocate
results := Array.with_capacity(1000)  # Reserve space upfront
for i in range(1000) {
    results.push(do_work(i))
}
```

### 3. Clear Large Data Early

```kujo
func process_file(path) {
    content := read_file(path)  # 100 MB
    result := parse(content)
    
    content := null  # Free 100 MB immediately
    
    # Rest of function with just 'result'
    return transform(result)
}
```

### 4. Avoid Deeply Nested Structures

**Bad** (high clone cost):
```kujo
tree := {
    "left": {
        "left": {
            "left": {"value": 1}
        }
    }
}

copy := tree  # O(n) clone of entire tree!
```

**Good** (flatten or use IDs):
```kujo
nodes := [
    { "id": 0, "left": 1, "right": 2 },
    { "id": 1, "left": 3, "right": 4 },
    # ...
]

# Clone is O(1) for IDs, not O(n) for structure
```

### 5. Use Generators for Large Sequences

**Bad** (allocates all values):
```kujo
nums := range(1000000)  # 1M Values allocated
for n in nums {
    if n > 100 { break }  # Wasted 999,900 allocations!
}
```

**Good** (lazy evaluation):
```kujo
nums := range(1000000)  # No allocation yet
for n in nums {
    if n > 100 { break }  # Only created 101 values
}
```

### 6. Limit Closure Captures

**Bad** (captures entire environment):
```kujo
func make_handler() {
    huge_data := load_huge_dataset()  # 1 GB
    counter := 0
    
    func handler() {
        counter := counter + 1  # Only needs counter!
        return counter
    }
    
    # But closure captures BOTH counter and huge_data!
    return handler
}
```

**Good** (minimize captures):
```kujo
func make_handler() {
    huge_data := load_huge_dataset()
    result := process(huge_data)
    huge_data := null  # Release before creating closure
    
    counter := 0
    func handler() {
        counter := counter + 1
        return counter
    }
    
    # Closure only captures counter now
    return handler
}
```

---

## Debugging Memory Issues

### Symptom 1: Slow Performance

**Possible Cause**: Excessive cloning.

**Diagnosis**:
```kujo
import time

start := time.now()
for i in range(1000) {
    copy := large_data  # Suspect: clone here?
    process(copy)
}
elapsed := time.now() - start

print("Time: ${elapsed}ms")  # If high, likely clone overhead
```

### Symptom 2: Increasing Memory Usage

**Possible Cause**: Memory leak (reference cycle or leaked closures).

**Diagnosis**:
- Check for circular references
- Verify closures don't capture too much
- Look for LeakyFunctionBody accumulation

### Symptom 3: Stack Overflow

**Possible Cause**: Deep recursion or LeakyFunctionBody issue.

**Diagnosis**:
```kujo
# Check recursion depth
func recursive(n, depth) {
    if depth > 1000 {
        print("ERROR: Too deep!")
        return
    }
    # ... recursive logic
    return recursive(n - 1, depth + 1)
}
```

---

## Future Improvements

### Roadmap Items

1. **Task #29**: Fix LeakyFunctionBody with iterative drop
2. **Task #30**: Separate AST from runtime values (no more LeakyFunctionBody!)
3. **Cycle Detection**: Detect and break reference cycles
4. **Tracing GC**: Optional tracing garbage collector for long-running programs
5. **Memory Profiler**: Built-in memory profiling (`kujo profile --memory`)

### Potential Optimizations

- **Specialized Collections**: IntArray, FloatArray (no Value overhead)
- **Small Value Optimization**: Box large variants, keep small variants inline
- **Copy-on-Write**: Clone only when mutated
- **Arena Allocator**: Batch allocate related values

---

## Further Reading

- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture overview
- [CONCURRENCY.md](CONCURRENCY.md) - Thread safety and concurrency
- [EXTENDING.md](EXTENDING.md) - Adding native functions
- [ROADMAP.md](../ROADMAP.md) - Planned memory improvements

---

**Questions?** Open an issue on GitHub or check the documentation.
