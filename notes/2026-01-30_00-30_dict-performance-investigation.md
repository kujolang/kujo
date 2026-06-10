# Kujo Performance Investigation Session - 2026-01-30

## Objective
Investigate and fix performance issues where Kujo is slower than Python, particularly in hash map operations.

## Key Findings

### 1. Dict/HashMap Performance Bottleneck (CRITICAL)

**Problem**: Hash map operations are **1300x slower than Python**
- Kujo: 373ms for 1000 dict operations
- Python: 0.28ms for same operations

**Root Cause**: Excessive Value cloning in VM operations

The bottleneck is in `src/vm.rs` Store operations:

```rust
// Line 482 - StoreVar peeks and CLONES the entire Value
let value = self.stack.last().ok_or("Stack underflow")?.clone();

// Line 509 - StoreGlobal also clones
let value = self.stack.last().ok_or("Stack underflow")?.clone();
```

Since `Value` implements `#[derive(Clone)]`, this deep-clones entire HashMaps/Vecs on EVERY assignment!

**Sequence for `map[i] := value`:**
1. LoadVar(map) - clones map from locals
2. Push index
3. Push value  
4. IndexSet - pops 3, modifies, pushes modified map
5. StoreVar(map) - **CLONES** map, stores it
6. Pop - removes map from stack

Result: **2 full HashMap clones per assignment** (LoadVar + StoreVar)

### 2. "test" Keyword Bug

**Problem**: Functions named `test()` cause compiler to hang/freeze

**Root Cause**: `test` is a reserved keyword (line 240 in lexer.rs)

```rust
| "test" | "test_setup" | "test_teardown" | "test_group" => TokenKind::Keyword(ident),
```

When used as a function name, parser enters infinite loop trying to parse it as a keyword construct.

**Status**: Already documented in `notes/GOTCHAS.md`

**Workaround**: Use different function names (my_func, hash_map_ops, etc.)

### 3. Store Optimization Attempt (INCOMPLETE)

**Approach**: Change StoreVar/StoreGlobal from peek+clone to pop semantics

**Changes Made**:
1. `src/vm.rs` lines 482, 509: Changed `.last()?.clone()` to `.pop()?`
2. `src/compiler.rs` lines 111, 124: Removed explicit `Pop` after Let/Assign

**Result**: Stack underflow errors in functions

**Analysis**: The optimization is conceptually correct but breaks existing assumptions:
- Current design: StoreVar **peeks** (leaves value on stack), explicit Pop removes it
- Commit bc7e807: "fix: emit Pop after let/assign statements for correct stack hygiene"
- This design was intentional for JIT loop compilation

**Challenge**: Many code paths depend on Store leaving value on stack. Changing this requires:
- Audit all uses of StoreVar/StoreGlobal
- Determine which need the value to remain vs be consumed
- Potentially add StoreVarPop vs StoreVar opcodes

### 4. Alternative Solutions

**Option A: Rc<RefCell<>> for Collections** (Rejected - too invasive)
- Change `Array(Vec<Value>)` to `Array(Rc<RefCell<Vec<Value>>>)`
- Change `Dict(HashMap<...>)` to `Dict(Rc<RefCell<HashMap<...>>>)`
- Impact: 259 compilation errors across entire codebase
- Estimated effort: 2-3 weeks of refactoring

**Option B: Optimize Store Operations** (Current approach - incomplete)
- Change Store from peek+clone to pop
- Requires careful coordination with compiler
- Stack underflow issues need resolution

**Option C: Copy-on-Write (Alternative)**
- Wrap collections in Arc and clone only when modified
- Would require custom wrapper types
- More complexity but less invasive than Rc<RefCell<>>

## Benchmarks

### Dict Operations (n=1000)
- Simple loop baseline: 5ms
- Dict population: 161ms (32x slower than loop!)
- Dict lookup: 159ms (32x slower than loop!)
- Combined: 373ms

### Python Comparison (n=1000)
- Python: 0.28ms
- Kujo: 373ms
- **Kujo is 1330x SLOWER**

### Python Comparison (n=100k)
- Python: 83ms
- Kujo: Would take ~37 seconds (extrapolated)

## Next Steps

1. **Immediate**: Understand why Store optimization causes stack underflow
   - Add debug logging to trace stack state
   - Identify which operations expect value to remain on stack
   - Consider two-opcode approach: StorePop vs StorePeek

2. **Short-term**: Implement working Store optimization
   - Fix stack underflow issues
   - Add comprehensive tests
   - Benchmark improvements

3. **Long-term**: Consider Rc<RefCell<>> refactor for proper solution
   - This is the "right" way to handle mutable collections in Rust
   - Would eliminate all cloning overhead
   - Large effort but permanent fix

## Test Results

- Baseline tests: 195 passed, 3 failed (pre-existing failures)
- Store optimization tests: Stack underflow in function contexts
- Top-level code: Works correctly
- Function code: Fails with underflow

## Files Modified (Reverted)

- `src/vm.rs`: StoreVar/StoreGlobal implementation
- `src/compiler.rs`: Let/Assign Pop emission

## Lessons Learned

1. Always check if a symbol is a reserved keyword before using it as a function name
2. Rust's ownership model makes mutable collection optimization non-trivial
3. Stack-based VM requires careful coordination between VM operations and compiler code generation
4. Performance bottlenecks can be hidden in seemingly innocuous `.clone()` calls
5. Test with realistic data sizes - small tests (n=10) may not reveal performance issues

## Time Spent

~4 hours investigating, testing, and documenting

## Status

**Blocked**: Store optimization incomplete due to stack underflow issues
**Priority**: P0 - Dict performance is critical for real-world usage
**Next Session**: Debug stack underflow and complete Store optimization

---

**Session End**: 2026-01-30 01:30 AM
**Next Focus**: Resolve stack underflow in Store optimization or implement Copy-on-Write approach
