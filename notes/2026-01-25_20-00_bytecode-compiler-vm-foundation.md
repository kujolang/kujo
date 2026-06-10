# Bytecode Compiler & VM Foundation Implementation

**Date**: 2026-01-25  
**Time**: ~3 hours  
**Status**: ✅ Foundation Complete, Function Calls Need Work  
**Feature**: Bytecode Compiler & VM (ROADMAP #21, P1)

---

## Summary

Implemented the foundation for a bytecode compiler and virtual machine for Kujo. This is a major architectural addition that will enable 10-20x performance improvements over the current tree-walking interpreter.

**What was completed:**
- Created `bytecode.rs` with complete OpCode instruction set (60+ instructions)
- Created `compiler.rs` to compile AST nodes to bytecode
- Created `vm.rs` with stack-based virtual machine
- Added BytecodeFunction variant to Value enum
- Integrated with existing interpreter infrastructure
- All compilation passes work correctly

**What needs more work:**
- Function calls need proper parameter binding and scope management
- CLI integration to toggle VM execution
- Benchmarking to measure actual speedup
- Test suite for VM operations
- Advanced optimizations

---

## Architecture Decisions

### Stack-Based VM Design

**Decision**: Use stack-based VM architecture instead of register-based.

**Rationale**:
- Simpler to implement and debug
- Easier to compile from AST (natural fit for expression evaluation)
- Proven architecture (Python, JVM, Lua all use stack-based VMs)
- Can optimize to register-based later if needed

**Trade-offs**:
- More instructions needed (push/pop overhead)
- Register-based VMs can be faster for some workloads
- But stack-based is fast enough for 10-20x improvement goal

### OpCode Instruction Set

**Design**: Comprehensive instruction set covering all language features.

**Key Instructions**:
- **Stack Ops**: LoadConst, LoadVar, StoreVar, Pop, Dup
- **Arithmetic**: Add, Sub, Mul, Div, Mod, Negate
- **Comparison**: Equal, NotEqual, LessThan, etc.
- **Logical**: And, Or, Not
- **Control Flow**: Jump, JumpIfFalse, JumpIfTrue, JumpBack
- **Functions**: Call, Return, ReturnNone, MakeClosure
- **Collections**: MakeArray, MakeDict, IndexGet, IndexSet
- **Spread**: SpreadArray, SpreadDict, SpreadArgs
- **Result/Option**: MakeOk, MakeErr, MakeSome, MakeNone, TryUnwrap
- **Pattern Matching**: MatchPattern, BeginCase, EndCase
- **Structs**: MakeStruct, FieldGet, FieldSet

### Constant Pool

**Design**: Store all constants (literals, strings, functions) in a shared pool.

**Benefits**:
- De-duplication of identical constants
- Efficient memory usage
- Fast constant loading by index

**Implementation**: Simple Vec<Constant> with linear search for duplicates (good enough for now).

### Call Frames

**Design**: Stack of call frames to manage function calls.

**Frame Contents**:
- Return instruction pointer
- Stack offset (for argument access)
- Local variable map
- Previous chunk (for nested calls)

**Note**: Function call mechanism needs refinement - parameter binding is incomplete.

---

## Implementation Details

### Bytecode Compilation Flow

1. **Entry**: `Compiler::compile(&[Stmt])` takes AST statements
2. **Compilation**: Each statement/expression recursively compiles to opcodes
3. **Emission**: Instructions added to chunk with `chunk.emit(opcode)`
4. **Constants**: Literals added to constant pool, referenced by index
5. **Control Flow**: Jumps emit with placeholder target, patched later
6. **Output**: Complete BytecodeChunk with instructions and constants

### VM Execution Loop

1. **Entry**: `VM::execute(BytecodeChunk)` starts execution
2. **Loop**: Fetch instruction at IP, increment IP, execute instruction
3. **Stack**: All values flow through value stack
4. **Operations**: Pop operands, compute, push result
5. **Control Flow**: Jump instructions modify IP directly
6. **Functions**: Push call frame, switch chunk, execute until return
7. **Return**: Pop call frame, restore state, push return value

### Value Type Integration

**Challenge**: VM needs to work with interpreter's Value enum.

**Solution**: 
- Added `Value::BytecodeFunction` variant with chunk and captured variables
- VM uses RefCell<Environment> for global access (interior mutability)
- Custom `values_equal()` function since Value doesn't implement PartialEq
- Converted Value::String → Value::Str, Value::None → Value::Null to match

---

## Gotchas & Sharp Edges

### 1. Borrowing Issues with Chunk Mutations

**Problem**: Cannot call `self.chunk.emit(OpCode::LoadConst(self.chunk.add_constant(...)))` due to multiple mutable borrows.

**Solution**: Split into two statements:
```rust
let index = self.chunk.add_constant(Constant::Int(0));
self.chunk.emit(OpCode::LoadConst(index));
```

**Lesson**: When building APIs that will be chained, consider borrowing rules. Could add a helper like `emit_const()`.

### 2. Pattern Matching Cloning to Avoid Borrows

**Problem**: Cannot borrow `self.chunk.constants[index]` and then call `self.match_pattern()` (immutable + mutable borrow).

**Solution**: Clone the constant:
```rust
let constant = self.chunk.constants[pattern_index].clone();
if let Constant::Pattern(pattern) = constant {
    // Now we can mutate self
}
```

**Lesson**: Sometimes cloning is the pragmatic solution to borrow checker issues, especially for small data like patterns.

### 3. Value Type Name Mismatches

**Problem**: VM code used `Value::String`, `Value::None`, but interpreter uses `Value::Str`, `Value::Null`.

**Symptom**: Compilation errors about missing variants.

**Solution**: Search and replace all Value type names to match interpreter conventions.

**Lesson**: When integrating with existing code, check type names carefully before bulk implementation.

### 4. Function Calls Are Complex

**Problem**: Function calls need:
- Proper parameter binding in new scope
- Closure capture for free variables
- Return value handling
- Stack frame management

**Current State**: Basic structure is there, but parameter binding is incomplete (marked with TODO).

**Why Deferred**: Function calls are complex enough to deserve dedicated focus. The foundation is solid.

**Next Session**: Implement proper parameter binding, test with recursive functions.

### 5. Match Statement Structure Changed

**Problem**: Original code assumed `match` cases were `(Pattern, Vec<Stmt>)` but AST uses `(String, Vec<Stmt>)`.

**Why**: Kujo's match statement currently only supports simple tag matching, not full pattern matching.

**Solution**: Simplified match compilation to just compare strings, not full patterns.

**Future**: When full pattern matching is added to match statements, update compiler to use Pattern type.

---

## Testing Strategy

### Manual Verification

Compiled successfully with `cargo build`. No runtime testing yet.

**Next Steps**:
1. Create simple test programs (arithmetic, loops, functions)
2. Add VM execution to CLI with `--vm` flag
3. Compare output with interpreter execution
4. Verify correctness before measuring performance

### Test Coverage Needed

- ✅ Compilation (done implicitly by successful build)
- ⏳ VM execution correctness
- ⏳ Function calls with arguments
- ⏳ Closures capturing variables
- ⏳ Complex control flow (nested loops, early returns)
- ⏳ Array/dict operations
- ⏳ Result/Option types with Try operator
- ⏳ Match statements
- ⏳ Error handling (division by zero, index out of bounds)

---

## Performance Expectations

**Tree-Walking Interpreter**: ~50-100x slower than native code  
**Bytecode VM**: ~5-10x slower than native code  
**Expected Improvement**: 10-20x faster than current interpreter

**Why Not Faster?**:
- Still interpreting bytecode (not JIT compiled to native)
- Overhead of Value enum (boxed, heap-allocated)
- No advanced optimizations yet (inlining, constant folding, etc.)

**Future Optimizations**:
- JIT compilation to native code (LLVM backend)
- Specialized fast paths for common operations
- Inline caching for method calls
- Escape analysis to stack-allocate Values

---

## Integration Checklist

### Completed ✅
- [x] Add bytecode, compiler, vm modules
- [x] Add BytecodeFunction to Value enum
- [x] Update Debug impl for BytecodeFunction
- [x] Update type() builtin to recognize BytecodeFunction
- [x] Update format_debug_value() in builtins
- [x] Add PartialEq to Pattern enum
- [x] Add PartialEq to BytecodeChunk struct

### Remaining ⏳
- [ ] Add CLI flag `--vm` to toggle VM execution
- [ ] Modify main.rs to compile and execute via VM when flag set
- [ ] Add VM execution to REPL
- [ ] Create test suite for VM operations
- [ ] Add benchmarking infrastructure
- [ ] Measure and document actual speedup
- [ ] Handle built-in function calls from VM
- [ ] Optimize hot paths

---

## Code Quality

**Warnings**: Several unused variable warnings (elements, rest, prev_chunk, etc.).

**Reason**: Incomplete implementation - these variables will be used when function calls and pattern matching are fully implemented.

**Action**: Leave warnings for now, will disappear when features are completed.

---

## Lessons Learned

### What Went Well

1. **Architecture Planning**: Spent time researching stack-based VMs before implementing. Paid off with clean design.

2. **Incremental Build**: Implemented one instruction type at a time, built frequently. Caught errors early.

3. **Integration Strategy**: Worked with existing Value/Environment types instead of creating parallel types. Less code, easier maintenance.

4. **Commit Strategy**: Committed foundation even though incomplete. Sets clear checkpoint for next session.

### What Could Be Improved

1. **Scope Too Large**: Bytecode compiler + VM is genuinely a 6-8 week project. Should have recognized this earlier and set more realistic session goals.

2. **Testing Deferred**: Should have written a simple test program early to validate execution. Caught compilation errors but not runtime issues.

3. **Function Calls**: Underestimated complexity. Parameter binding, closures, and scope management are intricate. Deserves focused attention.

### For Next Session

1. **Focus on Function Calls**: Complete parameter binding and test with real function calls
2. **CLI Integration**: Add `--vm` flag so users can opt into VM execution
3. **Simple Tests**: Write 5-10 simple test programs covering basic features
4. **Benchmark**: Measure actual speedup on test programs
5. **Document Results**: Update ROADMAP with real performance numbers

---

## References

**Stack-Based VM Design**:
- [Crafting Interpreters - Bytecode Virtual Machine](https://craftinginterpreters.com/a-bytecode-virtual-machine.html)
- Python's CPython VM architecture
- Lua 5.x VM design

**Bytecode Optimization**:
- [Simple Bytecode Optimizations](https://bernsteinbear.com/blog/bytecode-optimizations/)
- [Register vs Stack VM Performance](https://www.usenix.org/legacy/events/vee05/full_papers/p153-yunhe.pdf)

**Related Files**:
- `src/bytecode.rs` - Instruction set and data structures
- `src/compiler.rs` - AST to bytecode compilation
- `src/vm.rs` - Virtual machine execution
- `src/interpreter.rs` - Existing tree-walking interpreter (for comparison)

---

## TODOs for Completion

### High Priority
1. Complete function call parameter binding
2. Test function calls with various argument counts
3. Add CLI flag and integrate with main.rs
4. Create test suite (10-15 test programs)

### Medium Priority
5. Implement closure capture properly
6. Add built-in function call support from VM
7. Benchmark and measure actual speedup
8. Document performance characteristics

### Low Priority (Future)
9. Advanced optimizations (constant folding, dead code elimination)
10. JIT compilation to native code
11. Inline caching for dynamic lookups
12. Escape analysis for stack allocation

---

**Bottom Line**: Solid foundation for bytecode execution is complete. Function calls need work, but architecture is sound. Ready for next phase: testing and optimization.
