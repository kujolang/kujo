# Kujo Field Notes — Phase 4C: JIT Specialized Method Integration

**Date:** 2026-01-27  
**Session:** 11:02 PST  
**Branch/Commit:** main / 0352c74  
**Scope:** Integrated specialized arithmetic methods into JIT compiler's translate_instruction(). Connected Phase 4B specialized methods to actual compilation flow with specialization context checks.

---

## What I Changed

- **Modified 4 arithmetic operations** in `translate_instruction()` (src/jit.rs lines 545-580):
  - `OpCode::Add` - Check specialization, route to translate_add_specialized or generic
  - `OpCode::Sub` - Check specialization, route to translate_sub_specialized or generic
  - `OpCode::Mul` - Check specialization, route to translate_mul_specialized or generic
  - `OpCode::Div` - Check specialization, route to translate_div_specialized or generic
- **Integration pattern**: `if self.specialization.is_some() { specialized() } else { generic() }`
- **Added 3 new integration tests**:
  - `test_compilation_with_specialization_context` - Verify specialized path is chosen
  - `test_compilation_without_specialization_fallback` - Verify generic path works
  - `test_all_arithmetic_ops_with_specialization` - All 4 ops with profiles
- **Updated documentation**:
  - `CHANGELOG.md` - Added Phase 4C section (85% complete)
  - `ROADMAP.md` - Updated to show Phases 4A-C complete, 4D-E remaining
  - `README.md` - Phase 4 progress updated with 4C completion
- **Created 2 atomic commits**:
  1. `:package: NEW: Phase 4C integrate specialized methods into JIT`
  2. `:book: DOC: update documentation for Phase 4C completion`

---

## Gotchas (Read This Next Time)

### Gotcha 1: Cannot recompile same offset multiple times in one test

- **Symptom:** Test failed with "Duplicate definition of identifier: kujo_jit_100"
- **Root cause:** Cranelift's JITModule maintains function definitions by name. Compiling offset 100 twice creates `kujo_jit_100` function twice, which fails.
- **Fix:** Use different offsets for each compilation in the same test (100, 200, 300, 400)
- **Prevention:**
  - Each `compiler.compile(&chunk, offset)` creates a unique function name based on offset
  - In tests that compile multiple chunks, use different offsets
  - Or create a new JitCompiler instance for each compilation
  - This is a Cranelift JITModule limitation, not a Kujo bug

### Gotcha 2: Borrow checker error with inline constant creation

- **Symptom:** `cannot borrow 'chunk' as mutable more than once at a time` when calling `chunk.emit(OpCode::LoadConst(chunk.add_constant(...)))`
- **Root cause:** `add_constant` takes `&mut self` and returns a value. `emit` also takes `&mut self`. Can't have two mutable borrows simultaneously.
- **Fix:** Split into two statements:
  ```rust
  let const_id = chunk.add_constant(Constant::Int(42));
  chunk.emit(OpCode::LoadConst(const_id));
  ```
- **Prevention:** 
  - Always split constant creation from emission
  - This is standard Rust borrow checker behavior
  - Method chaining with multiple `&mut self` calls doesn't work

### Gotcha 3: Specialization checks happen at compilation, not runtime

- **Symptom:** You might think specialization is checked during execution
- **Reality:** `self.specialization` is checked during **JIT compilation** to decide which IR to generate. Once compiled, the native code runs without any specialization checks.
- **Rule:** The specialization decision is **compile-time**, not **runtime**.
- **Why:** This is the whole point - we want zero runtime overhead. Type checks happen via guards at function entry (Phase 4D), not per-operation.
- **Implication:** 
  - If a function is compiled with specialization, it always uses that path
  - If type assumptions become invalid, guards fail (Phase 4D) and function is deoptimized and recompiled
  - Can't dynamically switch between specialized and generic in the same compiled function

---

## Things I Learned

### Mental Model Updates

1. **Specialization is a compilation strategy, not an execution feature**
   - The `if self.specialization.is_some()` check runs during Cranelift IR generation
   - Generated native code is either specialized OR generic, never both
   - Runtime guards (Phase 4D) will detect assumption violations and trigger recompilation

2. **Generic fallback is essential**
   - Not all functions will have type profiles
   - New functions haven't been profiled yet
   - Mixed-type functions won't specialize
   - Generic path ensures all code can run, even without optimization

3. **Integration is cleaner than expected**
   - Simple `if/else` in translate_instruction
   - No complex dispatch logic needed
   - Specialized methods already handle the complexity
   - Clean separation of concerns

4. **Test counts tell a story**
   - Started Phase 4A: 17 JIT tests
   - After Phase 4B: 22 JIT tests (5 new)
   - After Phase 4C: 25 JIT tests (3 new)
   - Incremental test growth validates incremental development

### Rules of Thumb

- **Rule:** Each bytecode offset can only be compiled once per JitCompiler instance. Use different offsets or new instances.
- **Rule:** When writing tests, decide: am I testing compilation or execution? Phase 4B/C test compilation, Phase 3 tested execution.
- **Rule:** Specialization logic belongs in specialized methods, not in translate_instruction. Keep translate_instruction simple.
- **Rule:** Always provide a generic fallback. Specialization should be an optimization, not a requirement.

### Implicit Invariants

- **Invariant:** `translate_instruction()` is called once per bytecode instruction during compilation. The specialization check happens exactly once per instruction.
- **Invariant:** If `self.specialization.is_some()`, it means the JitCompiler found a stable type profile (60+ samples, 90%+ consistency) for this function offset.
- **Invariant:** Specialized methods and generic paths must produce semantically equivalent results, just with different performance characteristics.
- **Invariant:** The specialization context is set once at the start of compilation via `translator.set_specialization()` (line 1060 in src/jit.rs).

---

## Debug Notes

### Duplicate function definition error

**Failing test:** `test_all_arithmetic_ops_with_specialization`

**Error:**
```
Sub compilation failed: Some("Failed to define function: Duplicate definition of identifier: kujo_jit_100")
```

**Repro steps:**
1. Compile Add operation at offset 100
2. Compile Sub operation at same offset 100  
3. Second compilation fails with duplicate definition

**Root cause analysis:**
- Cranelift JITModule uses `format!("kujo_jit_{}", offset)` for function names
- Module maintains a symbol table and won't allow duplicate definitions
- Each `module.declare_function()` call must have a unique name

**Fix:**
```rust
// Instead of same offset for all:
let offset = 100;
compiler.compile(&add_chunk, offset);   // Creates kujo_jit_100
compiler.compile(&sub_chunk, offset);   // ERROR: kujo_jit_100 exists

// Use different offsets:
compiler.compile(&add_chunk, 100);  // Creates kujo_jit_100
compiler.compile(&sub_chunk, 200);  // Creates kujo_jit_200 ✓
compiler.compile(&mul_chunk, 300);  // Creates kujo_jit_300 ✓
compiler.compile(&div_chunk, 400);  // Creates kujo_jit_400 ✓
```

---

## Follow-ups / TODO (For Future Agents)

- [ ] **Phase 4D:** Guard generation at function entry
  - Insert type guards using `jit_check_type_int` and `jit_check_type_float`
  - Generate conditional branch: guards pass → specialized body, guards fail → deopt
  - Test guard success and failure scenarios

- [ ] **Deoptimization handler:**
  - Implement fallback when guards fail
  - Should evict from cache and mark for recompilation
  - Or fall back to interpreter execution
  - Test deopt → recompile → re-specialize cycle

- [ ] **Performance benchmarking:**
  - Create benchmark comparing specialized vs non-specialized execution
  - Measure the actual 2-3x speedup claim
  - Test various workloads: pure int, mixed types, variable-heavy

- [ ] **Float specialization completion:**
  - Research Cranelift bitcast/fcvt APIs properly
  - Implement float-specialized arithmetic paths
  - Test float operations with specialization

- [ ] **Enhanced integration tests:**
  - Add execution tests (not just compilation tests)
  - Validate that specialized code produces correct results
  - Compare results between specialized and generic paths

- [ ] **Optimize test function name generation:**
  - Consider adding test helper: `compiler.compile_unique(&chunk)` that auto-assigns unique offsets
  - Would simplify tests and prevent duplicate definition errors

---

## Links / References

### Files touched:
- `src/jit.rs` - Modified translate_instruction Add/Sub/Mul/Div (lines 545-580), added tests (lines 1935-2030)
- `CHANGELOG.md` - Phase 4C section
- `ROADMAP.md` - Phase 4 progress update to 85%
- `README.md` - Phase 4 summary with 4C completion

### Related docs:
- `.github/AGENT_INSTRUCTIONS.md` - Incremental commit guidelines
- `ROADMAP.md` Phase 4 section - Technical roadmap
- `notes/2026-01-27_10-53_phase4b-specialized-codegen.md` - Previous session notes

### Key code sections:
- `translate_instruction()` method (line 535) - Where integration happens
- Specialized methods (lines 874-970) - Called from translate_instruction
- `BytecodeTranslator::specialization` field (line 462) - Context checked during integration
- `JitCompiler::compile()` method (line 946) - Sets specialization context at line 1060

### Test files:
- Lines 1935-1970: New Phase 4C integration tests
- Lines 1630-1730: Phase 4B specialized operation tests
- Lines 1510-1650: Phase 3 execution tests

---

## Assumptions I Almost Made

1. **Almost assumed:** Specialization checks happen at runtime during code execution
   - **Reality:** Checks happen at compile-time during IR generation
   - **Learned:** Specialization is a compilation strategy, producing different native code

2. **Almost assumed:** Could compile the same offset multiple times to test different operations
   - **Reality:** Cranelift JITModule prevents duplicate function names
   - **Learned:** Each offset maps to one function; use different offsets in tests

3. **Almost assumed:** Borrow checker would allow `chunk.emit(chunk.add_constant(...))`
   - **Reality:** Can't have two simultaneous mutable borrows
   - **Learned:** Always split constant creation from usage - standard Rust pattern

4. **Almost assumed:** Integration would need complex dispatch logic
   - **Reality:** Simple if/else based on `self.specialization.is_some()` is sufficient
   - **Learned:** Keep integration simple; complexity belongs in specialized methods

---

## Progress Summary

**Phases Complete:**
- Phase 4A: Type profiling infrastructure ✅
- Phase 4B: Specialized method creation ✅  
- Phase 4C: Integration into compilation flow ✅

**Current State:**
- 25 JIT tests passing (8 new since Phase 4A start)
- 56 total library tests passing
- Specialized methods actively used during compilation
- Generic fallback ensures universal compatibility

**Remaining Work (15% of Phase 4):**
- Phase 4D: Guard generation and deoptimization
- Phase 4E: Advanced optimizations (constant propagation, loop unrolling)
- Performance validation and benchmarking

**Architecture Achievement:**
The integration is now complete: functions with stable type profiles automatically use specialized code paths, while maintaining backward compatibility through generic fallback. The foundation for 2-3x additional performance gains is in place.
