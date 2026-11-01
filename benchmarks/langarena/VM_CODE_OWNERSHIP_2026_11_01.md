# VM code-owner audit, accepted 782c712d

Source inspected: src/vm.rs, src/jit.rs, src/jit_disabled.rs,
src/interpreter/value.rs; pinned cranelift-jit 0.113.1 backend.rs/memory.rs.

VM owns Option<JitCompiler> privately. ensure_jit_compiler creates it on demand;
no compiler take/replacement or public compiler accessor exists. Compiled function
pointers are held in private compiled_functions, compiled_fn_info and inline_cache
maps; compiled-loop pointers remain within the compiler. Public forced compilation
returns Result<bool,String>, statistics return counts, not pointers.

VmExecutionSnapshot copies bytecode, stacks, frames, Values and environment owners;
it does not transfer compiled caches. GeneratorContinuation similarly retains
bytecode state, not native instruction pointers. Value::BytecodeFunction carries
bytecode plus captured Values/kinds; NativeFunction is a builtin name. No compiled
function value variant exists. VMContext borrows an owning VM during synchronous
native invocation, and generated calls return before the borrowing execute/call
operation returns. Raw-pointer misuse by external unsafe callers is not a lifetime
guarantee this audit can establish.

Candidate direction: VM destruction may consume its private compiler through an
explicit crate-private unsafe reclamation method. Keep public JitCompiler drop
behavior unchanged. Reclaim only after all native calls have returned, never on
JIT disabling, cache replacement, suspension or callback return. The compiler
must stay alive with embedded constant/shape/key storage through each invocation.
A future VM public compiler accessor or exported native-pointer value would
invalidate the private-ownership proof and must revisit this destruction policy.

Required tests before acceptance: mapping baseline and post-fix VM lifecycle;
live native invocation and repeated cached calls; returned bytecode functions and
snapshots used in another VM after original VM drop; disabled-JIT compiler reuse;
nested callbacks/recursion, generator suspension, errors/cancellation; JIT-free
build; existing VM/interpreter/JIT parity and all-50 checksum regression gates.
Public compiler mapping retention must remain unchanged as a compatibility control.

This is a source-level candidate rationale, not acceptance of a runtime change.
