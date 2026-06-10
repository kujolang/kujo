# Session Notes: Phase 7 Step 10 - Fast Argument Passing

**Date**: 2026-01-29
**Phase**: 7 (Function-Level JIT Implementation)
**Step**: 10 (originally "Value Unboxing", pivoted to Fast Argument Passing)

## Summary

Investigated and partially implemented optimizations for JIT recursive function calls.
Achieved ~20% speedup but identified fundamental architecture limitation.

## Changes Made

### 1. VMContext Extended
- Added `arg0`, `arg1`, `arg2`, `arg3` fields for fast parameter passing
- Added `arg_count` field
- All existing tests still pass

### 2. New Runtime Helper
- Added `jit_get_arg(ctx, index)` - reads parameter from VMContext.argN field
- Registered in JIT symbol builder

### 3. JIT Parameter Initialization Updated
- `initialize_parameter_slots()` now has optional `get_arg_func` parameter
- For functions with ≤4 int params, uses `jit_get_arg` instead of HashMap lookup

### 4. VM Call Paths Updated
- Inline cache fast path now sets VMContext.argN fields
- Slow path in OpCode::Call now sets VMContext.argN fields  
- `call_function_from_jit` now sets VMContext.argN fields
- Empty HashMap used for simple integer functions (no population overhead)

### 5. Eliminated var_names Clone
- Changed from `cached.clone()` to using pointer to cached var_names
- Significant reduction in allocation for recursive calls

## Performance Results

| Version | fib(25) Time | Improvement |
|---------|--------------|-------------|
| Before (Step 9) | ~1.03s | baseline |
| After (Step 10) | ~0.81s | ~20% faster |
| Python | ~0.028s | 29x faster than Kujo |

## Attempted but Failed

### Direct JIT Self-Recursion
Attempted to make recursive calls within JIT without going through VM:
1. Added `current_function_name` and `self_recursion_pending` to BytecodeTranslator
2. Detected LoadVar of own function name
3. In Call opcode, tried to emit direct Cranelift function call

**Problem**: Stack overflow because:
- JIT function signature is `fn(*mut VMContext) -> i64`
- Arguments passed via stack slots are SHARED between caller and callee
- Recursive call overwrites parent's slot values before parent reads them

**Solution needed**: Change signature to `fn(*mut VMContext, arg0: i64, ...) -> i64`
This requires significant refactoring of the JIT architecture.

## Bottleneck Analysis

Current call flow for `fib(25)` (~242,785 calls):
```
JIT code
  → jit_call_function (FFI crossing)
    → pop args from stack
    → pop function from stack
    → call_function_from_jit
      → lookup compiled_fn
      → create VMContext
      → set arg fields
      → call compiled_fn (JIT)
        → jit_get_arg (FFI crossing)
        → computation
        → jit_set_return_int (FFI crossing)
      → extract return value
    → push result to stack
  ← return
```

Each recursive call crosses the FFI boundary 3+ times.
Python's interpreter is highly optimized for function calls.

## Next Steps

**Step 12: Direct JIT Recursion (P0 - CRITICAL)**
- Change JIT function signature to: `fn(*mut VMContext, arg0: i64) -> i64`
- Detect self-recursive calls at JIT compile time
- Emit direct Cranelift `call` instruction for self-recursion
- Expected speedup: 30-50x (would match or beat Python)

**Alternative Approach: Trampolining**
- Convert recursive calls to iteration with explicit stack
- More complex but doesn't require signature change

## Files Modified

- `src/jit.rs`:
  - VMContext struct (added argN fields)
  - Added `jit_get_arg()` helper
  - Updated `initialize_parameter_slots()` for fast args
  - Registered `jit_get_arg` symbol

- `src/vm.rs`:
  - Updated inline cache fast path
  - Updated slow path in OpCode::Call
  - Updated `call_function_from_jit`
  - All paths now set VMContext.argN fields

- `CHANGELOG.md`: Documented Step 10 progress
- `ROADMAP.md`: Updated status and added Step 12

## Tests

All 198 interpreter tests passing.
fib(25) produces correct result (75025).
