# Kujo Field Notes — Compiler Stack Pop Fix for Loop JIT

**Date:** 2026-01-28
**Session:** 18:50 local
**Branch/Commit:** main / (post-push)
**Scope:** Fixed critical compiler bug where `let` and `assign` statements weren't popping the stack after `StoreVar`, breaking JIT compilation for all functions containing loops.

---

## What I Changed

- Modified `src/compiler.rs` to emit `Pop` after `Stmt::Let` pattern bindings (lines ~102-113)
- Modified `src/compiler.rs` to emit `Pop` after `Stmt::Assign` assignments (lines ~116-128)
- Both changes ensure the stack is clean after each statement

### Before (broken):
```rust
Stmt::Let { pattern, value, .. } => {
    self.compile_expr(value)?;
    self.compile_pattern_binding(pattern)?;
    // NO POP - value stayed on stack!
}
```

### After (fixed):
```rust
Stmt::Let { pattern, value, .. } => {
    self.compile_expr(value)?;
    self.compile_pattern_binding(pattern)?;
    self.chunk.emit(OpCode::Pop);  // Clean up after statement
}
```

---

## Gotchas (Read This Next Time)

- **Gotcha:** `StoreVar` uses PEEK semantics, not POP
  - **Symptom:** Loop JIT failed with "Translation failed at PC X: Stack empty" at loop headers
  - **Root cause:** `StoreVar` PEEKS the top of stack (stores it but doesn't remove it). The compiler wasn't emitting `Pop` after `let`/`assign` statements, so values accumulated on the stack.
  - **Fix:** Added `self.chunk.emit(OpCode::Pop)` after every `compile_pattern_binding()` and `compile_assignment()` call
  - **Prevention:** Remember: **StoreVar = PEEK, not POP**. Every statement that uses StoreVar needs an explicit Pop to maintain stack hygiene.

- **Gotcha:** Loop JIT requires consistent stack state between entry and iteration
  - **Symptom:** First iteration of loop worked, but JumpBack failed because stack had extra values
  - **Root cause:** SSA block parameters for loop headers expect the same number of values on each entry. Stack corruption from missing Pops caused mismatch.
  - **Fix:** Same as above - clean stack after each statement
  - **Prevention:** When debugging loop JIT, use `DEBUG_JIT=1` and look for stack state differences between "Loop header" and "JumpBack" instructions

- **Gotcha:** The peephole optimizer can mask the real problem
  - **Symptom:** Initial debugging focused on optimizer's `Dup` insertion before `StoreVar+LoadVar` patterns
  - **Root cause:** The optimizer was doing something reasonable (optimizing `StoreVar x; LoadVar x` to `Dup; StoreVar x`), but it made the stack corruption worse and more visible
  - **Fix:** Fixed the underlying compiler issue, not the optimizer
  - **Prevention:** When optimizer-related bugs appear, check if the issue is in bytecode generation BEFORE optimization

---

## Things I Learned

- **Stack discipline is critical for JIT**: The interpreter can tolerate some stack messiness because it cleans up between statements. The JIT compiler generates native code that assumes precise stack states at every instruction boundary.

- **StoreVar semantics**: In Kujo's bytecode:
  - `StoreVar` = PEEK (stores TOS but leaves it on stack)
  - `StoreLocal` = PEEK (same)
  - This design allows chained assignments (`a = b = c = 1`) to work naturally
  - BUT it means explicit Pop is needed after standalone assignments

- **Loop header invariant**: For Cranelift JIT, loop headers are SSA blocks with parameters. The number and types of parameters are determined by the stack state at the FIRST entry. Every subsequent entry (via JumpBack) must have the SAME stack state.

- **Why the interpreter didn't catch this**: The interpreter likely has implicit cleanup or the stack grows without immediate failure. Only JIT exposes the corruption because it generates static code assuming exact stack states.

---

## Debug Notes

- **Failing test / error:** 
  ```
  JIT failed for function loop_test: Translation failed at PC 4: Stack empty
  ```

- **Repro steps:** 
  1. Create any function with a loop: `fn test() { let i = 0; while i < 10 { i = i + 1; } }`
  2. Call it 100+ times to trigger JIT
  3. Watch it fall back to interpreter

- **Breakpoints / logs used:**
  - `DEBUG_JIT=1` environment variable
  - Added `eprintln!` in `translate_instruction` for stack state tracking
  - Key insight came from seeing stack depths diverge between loop entry paths

- **Final diagnosis:** Stack corruption from missing Pop after let/assign statements. The first `let i = 0` left `0` on the stack. Each statement added more debris. By the time JumpBack executed, stack was polluted.

---

## Performance Results After Fix

| Benchmark | Kujo (JIT) | Python | Go | vs Python | vs Go |
|-----------|------------|--------|-----|-----------|-------|
| fib(25) | 0.54ms | 35.45ms | 0.40ms | **66x faster** | 1.3x slower |
| fib(30) | 6.14ms | 323ms | 4.68ms | **53x faster** | 1.3x slower |
| array_sum(100k) | 0.20ms | 10.36ms | 0.033ms | **52x faster** | 6x slower |
| nested_loops(500) | 0.36ms | 24.13ms | 0.11ms | **68x faster** | 3.3x slower |

**Before fix:** Loops ran at interpreter speed (~1800ms for array_sum, ~3600ms for nested_loops)
**After fix:** Loops JIT-compile correctly, running at native speed

---

## Follow-ups / TODO (For Future Agents)

- [ ] Consider adding a bytecode verifier pass that checks stack balance per basic block
- [ ] Add integration test specifically for loop JIT (ensure loops trigger JIT, not just functions)
- [ ] Document StoreVar PEEK semantics in code comments for future maintainers
- [ ] Consider if other statement types might have similar stack hygiene issues

---

## Assumptions I Almost Made

- **Wrong assumption:** "The optimizer is inserting bad Dup instructions" — Spent time looking at optimizer when the real issue was upstream in the compiler
- **Wrong assumption:** "StoreVar pops the stack like most stack machines" — Kujo's StoreVar PEEKS, which is valid but requires explicit Pop
- **Wrong assumption:** "If interpreter works, bytecode is correct" — Interpreter may be more forgiving; JIT exposes latent bugs

---

## Links / References

- Files touched:
  - `src/compiler.rs` (added Pop after Stmt::Let and Stmt::Assign)
- Related docs:
  - `docs/VM_INSTRUCTIONS.md` (StoreVar semantics)
  - `ROADMAP.md` (Step 11: Loop JIT now truly complete)
- Related notes:
  - `2026-01-28_18-32_step11-loop-jit-fix.md` (earlier session, partial fix)
