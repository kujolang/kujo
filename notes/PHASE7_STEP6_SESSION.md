# Phase 7 Step 6 Session Summary

## Date: 2026-01-29

## Goal
Implement recursive function JIT support for Kujo language to achieve 5-10x performance improvement over Python.

## Accomplishments

### 1. Fixed SSA Block Parameter Issues
- JumpIfFalse/JumpIfTrue were popping instead of peeking - fixed
- Blocks were created but never populated with correct function_end calculation
- Stack values now properly passed as block parameters

### 2. Fixed Comparison Operations
- LessEqual and GreaterEqual were using bnot which inverts all bits incorrectly
- Changed to use proper IntCC::SignedLessThanOrEqual and SignedGreaterThanOrEqual

### 3. Fixed Recursive Call Support
- LoadVar now handles function values (non-Int) by pushing to VM stack
- jit_call_function pops args first, then function (JIT stack order)
- Call opcode properly manages both JIT and VM stacks
- var_names registration includes all LoadVar targets in bytecode

### 4. Fixed Deadlock on Recursive Calls
- Mutex lock on globals was held during JIT execution
- Recursive calls would try to acquire the same lock
- Fixed by dropping lock before executing compiled function

### 5. Added var_names Caching
- Cache var_names HashMap per function to avoid re-hashing on every call
- Reduced overhead but still not enough for performance goals

## Current State

### Correctness: ✅ WORKING
- fib(10) = 55 ✓
- fib(25) = 75025 ✓
- All recursive calls execute correctly

### Performance: ❌ NEEDS IMPROVEMENT
- Kujo JIT fib(25): 1.3 seconds
- Kujo Interpreter fib(25): 1.2 seconds
- Python fib(25): 0.04 seconds

Kujo is ~33x slower than Python, not faster.

## Root Cause Analysis

The JIT implementation has too much overhead per function call:

1. **Runtime calls for every variable load**: jit_load_variable is a C ABI call with HashMap lookup
2. **Runtime calls for every function call**: jit_call_function recreates locals, VMContext
3. **Value boxing/unboxing**: Converting between i64 and Value enum
4. **No register allocation**: All values go through memory/HashMap
5. **No inlining**: Each recursive call goes through full call machinery

## What a Proper JIT Would Need

For fibonacci to be fast, the JIT should:

1. **Keep 'n' in a CPU register** throughout the function
2. **Do comparison inline**: `n <= 1` should be a single CMP instruction
3. **Make direct native calls** for recursion (or inline them)
4. **Use native stack** for call frames, not VM stack manipulation
5. **Return value in register**, not through VM stack

## Recommendations for Next Steps

### Short Term (Step 7-8)
1. Implement iterative fibonacci JIT - likely faster since loops don't have call overhead
2. Benchmark other patterns (array operations, etc.)

### Medium Term (Step 9-10)
1. Profile to find exact bottlenecks
2. Optimize hot paths (maybe inline simple functions)
3. Consider specialized compilation for integer-only functions

### Long Term (Future Phase)
1. Rewrite JIT to use register-based locals for simple functions
2. Implement proper call ABI (parameters in registers)
3. Add inlining pass for small functions
4. Consider moving to a tracing JIT approach

## Files Modified

- [src/jit.rs](src/jit.rs) - Fixed SSA handling, comparison ops, LoadVar, Call opcode
- [src/vm.rs](src/vm.rs) - Fixed deadlock, added var_names cache

## Test Files

- `test_verifier.kujo` - Simple fib(10) test
- `benchmark_fib.kujo` - Performance benchmark

## Commit Ready

The code is working correctly. Performance optimization is a separate effort.
Recommend committing: ":package: NEW: recursive function JIT support (correctness complete)"
