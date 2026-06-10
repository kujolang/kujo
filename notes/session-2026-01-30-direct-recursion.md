# Session Notes: Direct JIT Recursion (Step 12) - 2026-01-30

## Summary

Implemented infrastructure for direct JIT recursion (Phase 7 Step 12) but the expected 
performance improvement was not achieved. The code compiles and runs correctly but the 
execution is still slow.

## What Was Implemented

### New Types (jit.rs)
- `CompiledFnWithArg`: Function pointer type `fn(*mut VMContext, i64) -> i64`
- `CompiledFnInfo`: Struct containing standard and direct-arg variants + metadata

### New Methods (JitCompiler)
- `compile_function_with_info()`: Compiles both standard and direct-arg variants
- `compile_function_with_direct_arg()`: Creates the direct-arg Cranelift function
- `function_has_self_recursion()`: Detects self-recursive call patterns in bytecode

### New Translation Method (BytecodeTranslator)
- `translate_direct_arg_instruction()`: Handles direct-arg mode translation
- Handles LoadVar (using direct parameter), Call (direct self-recursion), Return (direct value)

### VM Changes
- Added `compiled_fn_info: HashMap<String, CompiledFnInfo>` to VM struct
- Updated `call_function_from_jit()` to use direct-arg variant when available
- Added interpreter fast path to use direct-arg for single-int-arg calls

### Bug Fixes
- Fixed self-recursion detection to scan ALL bytecode (not stop at first Return)
  - Original code stopped at first Return, missing recursive calls in else branches

## Current State

### What Works
- Self-recursion detection correctly identifies recursive functions
- Direct-arg compilation succeeds (no errors)
- Direct-arg functions are being called (confirmed via debug output)
- Correctness is maintained (fib(30) = 832040 is correct)
- All 198 existing tests pass

### What Doesn't Work
- Performance is NOT improved - fib(30) still takes minutes instead of milliseconds
- The internal Cranelift recursive calls seem to still be slow

## Investigation Needed

### Hypothesis 1: Cranelift IR Generation Issue
The generated direct calls might not be truly direct. Need to:
- Enable `DEBUG_JIT_IR=1` to inspect the actual Cranelift IR
- Verify `self_func_ref` points to the correct function
- Check if there's any FFI crossing in the generated code

### Hypothesis 2: Missing Optimization
Cranelift might not be optimizing tail calls or the call graph properly.
Need to check Cranelift settings for:
- Tail call optimization
- Inlining
- Loop optimization

### Hypothesis 3: Value Stack Pollution
The `translate_direct_arg_instruction` falls back to `translate_instruction`
for most opcodes, which might be introducing overhead or incorrect semantics.

## Files Changed

- `src/jit.rs`: ~500 lines added (types, methods, translation)
- `src/vm.rs`: ~100 lines added (interpreter fast path, info storage)
- `tests/jit_direct_recursion.kujo`: Comprehensive test suite
- `examples/benchmarks/bench_fib30.kujo`: Benchmark file

## Next Steps

1. Inspect generated Cranelift IR for fib function with `DEBUG_JIT_IR=1`
2. Verify the call instruction is truly a direct call (not through FFI)
3. Consider adding specialized path for entire direct-arg function (not mixing with standard translation)
4. Profile to identify where time is actually being spent
5. Check if Cranelift tail call optimization is applicable
