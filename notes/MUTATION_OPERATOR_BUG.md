# Critical Bug: Mutation Operator (`=`) is Completely Broken

**Status:** HISTORICAL - Superseded before v1.0 release readiness
**Impact:** 30-700% performance loss across benchmarks  
**Root Cause:** Compiler bug causing infinite loops when using mutation operator  
**Priority:** HIGH - Must fix before v1.0  

## Current Status - Superseded 2026-06-19

This note is a historical bug report and should not be treated as a current
v1.0 blocker. Current Kujo supports `=` assignment and compound assignment
as statement-level assignment operators, subject to binding mutability rules.
`:=` also updates an existing mutable binding when one is present.

Modern evidence:

- `docs/LANGUAGE_SPEC.md` lists `:=`, `=`, `+=`, `-=`, `*=`, `/=`, and `%=`
  as assignment statement operators and documents `let`/`mut`/`const`
  reassignment and in-place mutation behavior.
- `tests/language_spec_contracts.rs` verifies mutable rebinding and
  in-place mutation, plus immutable/const rejection.
- `tests/vm_interpreter_parity_surfaces.rs` verifies loop mutation, local map
  updates, nested/captured map updates, and named nested capture mutation in
  both interpreter and VM paths.
- On 2026-06-19, the historical repro `mut i := 0; while i < 5 { i = i + 1 };
  print(i)` printed `5` in both `kujo run` and `kujo run --interpreter`.

Keep the remaining sections for historical debugging context only.

## Executive Summary

Kujo's mutation operator (`=`) does not work correctly and causes infinite loops. This forces all code to use the declaration operator (`:=`) instead, which creates new variable bindings that shadow old ones rather than mutating them. This shadowing pattern keeps old `Arc<T>` references alive, preventing copy-on-write optimizations from working, resulting in 7-700% performance penalties.

**Performance Impact:**
- String concatenation: **7.0x slower than Python** (should be competitive)
- Hash map operations: **1.7x slower than Python** (should be faster)
- All optimizations relying on unique Arc ownership are ineffective

## Problem Demonstration

### Test Case: Simple Mutation

```kujo
# This should work but doesn't
i := 0
while i < 5 {
    print(i)        # Prints 0, 0, 0, 0, 0... FOREVER
    i = i + 1       # Does NOT mutate i - infinite loop!
}
```

**Expected:** Prints 0, 1, 2, 3, 4  
**Actual:** Prints 0 forever, must be killed with timeout  

### Test Case: Function Scope

```kujo
func test_mutation() {
    counter := 0
    print(counter)    # Prints: 0
    counter = counter + 1
    print(counter)    # Never executes - hangs forever
    return counter
}

test_mutation()  # Function hangs and never returns
```

**Expected:** Prints 0, then 1, returns 1  
**Actual:** Prints 0, then hangs forever  

### Test Case: String Concatenation

```kujo
result := ""
i := 0
while i < 10 {
    result := result + "x"   # MUST use := because = is broken
    i := i + 1               # MUST use := because = is broken
}
# result now has 10 x's BUT created 11 Arc<String> instances
# (1 original + 10 shadowed versions still in memory)
```

**Why This Kills Performance:**
1. Each `:=` creates a NEW `Arc<String>` with the concatenated value
2. Old `Arc<String>` is shadowed but not dropped (stays in memory)
3. `Arc::strong_count()` is always > 1 because old Arcs exist
4. Copy-on-write optimizations can never trigger
5. Every concatenation clones the entire string

## Root Cause Analysis

### Location: `src/compiler.rs`

The mutation operator is handled in two places:

#### 1. Assignment Statement Compilation (Line ~167-204)
```rust
Stmt::Assign { target, value } => {
    // Compile the value
    self.compile_expr(value)?;

    // Compile the assignment target
    self.compile_assignment(target)?;
    
    // Pop the value from stack
    self.chunk.emit(OpCode::Pop);
    
    Ok(())
}
```

#### 2. Assignment Target Resolution (Line ~1169-1230)
```rust
fn compile_assignment(&mut self, target: &Expr) -> Result<(), String> {
    match target {
        Expr::Identifier(name) => {
            if self.is_upvalue(name) {
                self.chunk.emit(OpCode::StoreVar(name.clone()));
            } else if let Some(slot) = self.resolve_local_slot(name) {
                // ← This path is taken for local mutation
                self.chunk.emit(OpCode::StoreLocal(slot));
            } else if self.scope_depth == 0 {
                self.chunk.emit(OpCode::StoreGlobal(name.clone()));
            } else {
                // This creates a NEW local instead of mutating!
                let slot = self.add_local(name, self.scope_depth);
                self.chunk.emit(OpCode::StoreLocal(slot));
            }
            Ok(())
        }
        // ... other cases
    }
}
```

### The Bug Mechanism

When `i = i + 1` is compiled:

1. **RHS Compilation:** `i + 1`
   - Compiler resolves `i` using `resolve_local_slot("i")`
   - Finds existing local at slot 0
   - Emits: `LoadLocal(0)`, `LoadConst(1)`, `Add`
   - Stack: `[old_value + 1]`

2. **LHS Compilation:** Assignment to `i`
   - Compiler calls `compile_assignment(&Expr::Identifier("i"))`
   - Calls `resolve_local_slot("i")` - **finds existing slot 0**
   - Emits: `StoreLocal(0)`
   - **Should work!** But there's a hidden bug...

3. **Hidden Bug:** Somewhere in the VM execution or bytecode sequence, the local variable isn't being properly updated in the loop context. Further investigation needed on these areas:
   - `OpCode::StoreLocal` implementation (src/vm.rs ~1047)
   - Loop bytecode structure (while loop compilation)
   - Stack/frame management during loops
   - Possible interaction with JIT compilation

### Alternate Theory: Scope Depth Issue

Looking at `resolve_local_slot`:
```rust
fn resolve_local_slot(&self, name: &str) -> Option<usize> {
    self.locals
        .iter()
        .rev()
        .find(|local| local.name == name && local.depth <= self.scope_depth)
        .map(|local| local.slot)
}
```

This searches from the end and checks `local.depth <= self.scope_depth`. If there's a scope depth mismatch in loop bodies, it might not find the variable or find the wrong one.

## Attempted Fixes (That Failed)

### Attempt 1: Slot Reuse for Shadowing

**Approach:** Modified `compile_pattern_binding` to reuse slots when shadowing at the same depth.

```rust
fn resolve_or_add_local(&mut self, name: &str, depth: usize) -> usize {
    // Check if name exists at SAME depth - reuse slot
    for local in self.locals.iter().rev() {
        if local.name == name && local.depth == depth {
            return local.slot;  // Reuse!
        }
    }
    self.add_local(name, depth)
}
```

**Result:** 2.5x performance **REGRESSION**  
- Fib Recursive: 5.68ms → 14.36ms (2.5x slower)
- String Concat: 7.73ms → 13.86ms (1.8x slower)
- Hash Maps: 48.01ms → 70.66ms (1.5x slower)

**Why it failed:** Reusing slots breaks JIT assumptions and causes incorrect variable lifetime management.

### Attempt 2: Arc::make_mut() Everywhere

**Approach:** Always use `Arc::make_mut()` even when `strong_count() > 1`, with aggressive capacity pre-allocation.

**Result:** Slight improvement but still 7x slower than Python
- String Concat: 11.47ms → 7.89ms (30% improvement)
- Still **7.0x slower than Python** (Python: 1.13ms)

**Why it helped:** Reduces allocations but can't eliminate the fundamental cloning issue.

## Correct Fix Needed

### Phase 1: Diagnostic Instrumentation

Add debug logging to understand bytecode execution:

```rust
// In src/vm.rs, OpCode::StoreLocal handler
OpCode::StoreLocal(slot) => {
    let value = self.stack.last().ok_or("Stack underflow")?.clone();
    
    if std::env::var("DEBUG_MUTATION").is_ok() {
        eprintln!("StoreLocal({}): storing {:?}", slot, value);
        eprintln!("  Frame locals before: {:?}", frame.local_slots);
    }
    
    let frame = self.call_frames.last_mut()
        .ok_or("StoreLocal requires call frame")?;
    if let Some(target) = frame.local_slots.get_mut(slot) {
        *target = value;
    } else {
        return Err(format!("Invalid local slot: {}", slot));
    }
    
    if std::env::var("DEBUG_MUTATION").is_ok() {
        eprintln!("  Frame locals after: {:?}", frame.local_slots);
    }
}
```

Run tests:
```bash
DEBUG_MUTATION=1 ./target/release/kujo run tmp/test_simple_mutation.kujo
```

### Phase 2: Bytecode Analysis

Dump bytecode for working (`:=`) vs broken (`=`) cases:

```kujo
# working.kujo
func test() {
    i := 0
    while i < 5 {
        print(i)
        i := i + 1  # Declaration - WORKS
    }
}

# broken.kujo  
func test() {
    i := 0
    while i < 5 {
        print(i)
        i = i + 1   # Mutation - HANGS
    }
}
```

Compare bytecode output to find differences.

### Phase 3: Fix the Core Issue

Based on diagnostic findings, likely fixes:

#### Option A: Loop Variable Handling
If loop variables aren't being properly managed:

```rust
// In compile_stmt for While loops
Stmt::While { condition, body, .. } => {
    let loop_start = self.chunk.instructions.len();
    self.loop_starts.push(loop_start);
    self.loop_ends.push(Vec::new());
    
    // Create a new scope for loop body?
    self.scope_depth += 1;  // Try this
    
    // Compile condition
    self.compile_expr(condition)?;
    let end_jump = self.chunk.emit(OpCode::JumpIfFalse(0));
    self.chunk.emit(OpCode::Pop);

    // Compile body
    for stmt in body {
        self.compile_stmt(stmt)?;
    }

    self.scope_depth -= 1;  // Exit loop scope
    
    // ... rest of loop handling
}
```

#### Option B: StoreLocal Implementation
If the VM's StoreLocal isn't working in loops:

```rust
OpCode::StoreLocal(slot) => {
    let value = self.stack.last().ok_or("Stack underflow")?.clone();
    let frame = self.call_frames.last_mut()
        .ok_or("StoreLocal requires call frame")?;
    
    // Ensure slot exists - resize if needed
    if slot >= frame.local_slots.len() {
        frame.local_slots.resize(slot + 1, Value::None);
    }
    
    frame.local_slots[slot] = value;
}
```

#### Option C: JumpBack Interaction
If JumpBack is resetting local slots:

Check if `OpCode::JumpBack` implementation is inadvertently resetting frame state.

### Phase 4: Test Suite

Create comprehensive tests:

```kujo
# tests/mutation_operator.kujo

# Test 1: Simple counter
assert_test("simple_counter", {
    i := 0
    i = i + 1
    i = i + 1
    return i == 2
})

# Test 2: Loop mutation
assert_test("loop_mutation", {
    sum := 0
    i := 0
    while i < 10 {
        sum = sum + i
        i = i + 1
    }
    return sum == 45 && i == 10
})

# Test 3: String mutation
assert_test("string_mutation", {
    s := ""
    i := 0
    while i < 5 {
        s = s + "x"
        i = i + 1
    }
    return len(s) == 5
})

# Test 4: Dict mutation
assert_test("dict_mutation", {
    d := {}
    i := 0
    while i < 5 {
        d[i] = i * 2
        i = i + 1
    }
    return len(d) == 5 && d[4] == 8
})

# Test 5: Nested mutations
assert_test("nested_mutations", {
    outer := 0
    i := 0
    while i < 3 {
        inner := 0
        j := 0
        while j < 3 {
            inner = inner + 1
            j = j + 1
        }
        outer = outer + inner
        i = i + 1
    }
    return outer == 9
})
```

### Phase 5: Verify Performance

After fixing, re-run benchmarks. Expected improvements:

| Benchmark | Current | Target | Expected Gain |
|-----------|---------|--------|---------------|
| String Concat | 7.89ms | ~1.0ms | **7.9x faster** |
| Hash Maps | 50.48ms | ~15ms | **3.4x faster** |
| Fib Recursive | 7.18ms | ~3ms | **2.4x faster** |

These gains come from:
1. No more shadowing overhead
2. `Arc::strong_count() == 1` optimizations work
3. True in-place mutations
4. Reduced memory allocations

## Technical Details

### Current Bytecode Pattern (Broken)

For `i = i + 1` in a loop:
```
Loop Start (ip: 100):
  LoadLocal(0)      # Load i (value: 0)
  LoadConst(1)      # Load 1
  Add               # Result: 1
  StoreLocal(0)     # Store to slot 0 ← Why doesn't this persist?
  JumpBack(100)     # Jump to loop start
```

### Expected Bytecode Pattern

Should be the same! The issue is in execution, not bytecode generation.

### Working Pattern (`:=` workaround)

For `i := i + 1`:
```
Loop Start (ip: 100):
  LoadLocal(0)      # Load i (value: 0)
  LoadConst(1)      # Load 1
  Add               # Result: 1
  StoreLocal(1)     # Store to NEW slot 1 ← Creates new variable
  # Old slot 0 still exists but is shadowed
  JumpBack(100)     # Jump to loop start
  # Next iteration loads from slot 0 again! (value: 0)
```

Wait... if `:=` creates a new slot, how does the loop work at all? The bug might be deeper than thought.

### Investigation Needed

1. **Trace exact bytecode**: Use bytecode dumper to see what's actually generated
2. **Trace execution step-by-step**: Log every opcode execution in the loop
3. **Check variable resolution**: Verify which slot is being loaded/stored
4. **Examine JIT behavior**: Does JIT compilation affect this differently?

## Related Code Locations

### Compiler
- `src/compiler.rs:167-204` - Stmt::Assign compilation
- `src/compiler.rs:1169-1230` - compile_assignment() method
- `src/compiler.rs:127-135` - resolve_local_slot() method
- `src/compiler.rs:113-125` - add_local() method
- `src/compiler.rs:240-270` - While loop compilation

### VM
- `src/vm.rs:1047-1059` - OpCode::StoreLocal implementation
- `src/vm.rs:687-698` - OpCode::LoadLocal implementation
- `src/vm.rs:777-825` - While loop execution (JumpBack handling)

### JIT
- `src/jit.rs` - Entire file may have issues with local variable handling

## Success Criteria

The fix is complete when:

1. ✅ `i = i + 1` increments the counter (doesn't hang)
2. ✅ All mutation tests pass
3. ✅ String concat performance matches Python (within 2x)
4. ✅ Hash map performance beats Python
5. ✅ No regressions in existing benchmarks
6. ✅ JIT compilation still works correctly

## Performance Target

After fixing the mutation operator, Kujo should achieve:

**7/8 wins vs Python** (87.5% win rate)

The only acceptable loss would be on highly optimized CPython built-ins where C-level optimizations are hard to beat.

## References

- Initial optimization: Commit d24099d "Optimize string concatenation to always use Arc::make_mut()"
- Previous optimizations: Commits 18647f6, 8a5583e
- Benchmark results: `benchmarks/cross-language/results/benchmark_20260211_*.txt`
- Test files: `tmp/test_simple_mutation.kujo`, `tmp/test_string_mutation.kujo`, etc.

## Next Steps for Implementation

1. Add DEBUG_MUTATION instrumentation
2. Create bytecode dumper for loops
3. Trace execution of both `:=` and `=` variants
4. Identify exact divergence point
5. Implement fix based on findings
6. Add comprehensive test suite
7. Verify performance gains
8. Update documentation

---

**Note for AI Agent:** This bug is CRITICAL. The mutation operator is a core language feature. Until this is fixed, Kujo cannot compete with interpreted languages on performance, despite having JIT compilation. All VM-level optimizations are neutered by this bug.
