# Kujo Field Notes — Channel Method Calls VM Bug Fix

**Date:** 2026-02-14
**Session:** 11:09 local
**Branch/Commit:** main / 24fa204
**Scope:** Fixed critical bug where channel methods (send/receive) failed with "Cannot access field on non-struct" error when JIT-compiled. Investigated concurrency roadmap status and updated documentation.

---

## What I Changed

- Fixed VM's `FieldGet` opcode to support Channel method calls (`src/vm.rs:4003-4048`)
- Modified `call_native_function_vm` to handle `__channel_method_` prefix for channel methods (`src/vm.rs:4873-4938`)
- Fixed `examples/concurrency_channels.kujo` string concatenation issues
- Updated `CHANGELOG.md` to document the bug fix
- Updated `ROADMAP.md` to mark Phase 1 concurrency items as complete
- Updated `README.md` channel example to show working usage without spawn isolation issues
- Made 5 commits with proper emoji prefixes (`:bug:`, `:ok_hand:`, `:book:`)

---

## Gotchas (Read This Next Time)

### Gotcha 1: VM FieldGet Only Supported Limited Types

- **Symptom:** Runtime error `Cannot access field on non-struct` when calling `chan.send(42)` or `chan.receive()`
- **Root cause:** VM's `OpCode::FieldGet` handler only supported Struct, Dict, FixedDict, IntDict, and DenseIntDict types. Channel was not handled.
- **Why it matters:** When code is JIT-compiled, it goes through the VM. The interpreter had separate Call expression handling for channel methods, but the VM bytecode path had no such handling.
- **Fix:** Added Channel case to FieldGet that:
  1. Pushes the channel object back onto stack
  2. Returns a `NativeFunction` with `__channel_method_{method_name}` marker
  3. The subsequent Call opcode sees this marker and handles it specially
- **Prevention:** When adding new Value types with methods, ensure both interpreter Call expression handling AND VM FieldGet/Call opcode handling support them.

### Gotcha 2: Stack Management for Method Calls in VM

- **Symptom:** Initial attempt at fixing caused stack ordering issues
- **Root cause:** When `FieldGet` pops the object, it needs to be available later for the method call. But the Call opcode expects function on top, args below.
- **Solution pattern:** For method calls on non-struct types:
  1. FieldGet pops the object
  2. Pushes the object back onto stack (below where the function marker will be)
  3. Pushes a special marker function (like `__channel_method_send`)
  4. Call opcode pops function marker, recognizes the prefix
  5. Pops the object from stack again
  6. Executes the method
- **Why this works:** The Call opcode collects args first (which don't include the object at this point), then we pop the object separately.
- **Prevention:** Always trace stack layout when modifying opcodes: [... | obj] -> FieldGet -> [... | obj | method_marker] -> LoadArg -> [... | obj | method_marker | arg] -> Call(1)

### Gotcha 3: Concurrency Roadmap Was Misleading

- **Symptom:** ROADMAP.md listed "implement spawn keyword" and "add channel()" as TODO items in Phase 1
- **Root cause:** These features were already implemented but the roadmap wasn't updated. The actual problem was the VM bug preventing them from working with JIT.
- **Discovery:** When investigating the "high priority feature", found that:
  - `spawn` keyword exists in lexer, parser, AST, and interpreter
  - `channel()` function exists and creates `Value::Channel`
  - `.send()` and `.receive()` methods exist in interpreter
  - The ONLY issue was VM bytecode execution path
- **Fix:** Updated ROADMAP to mark items complete and added status notes about current limitations
- **Prevention:** Always grep the codebase for claimed "missing" features before implementing from scratch. Check lexer keywords, parser functions, AST nodes, and interpreter handlers.

### Gotcha 4: Current Spawn Runs in Isolation

- **Symptom:** Example showing `spawn { chan.send(...) }` doesn't work - spawned thread can't access parent scope variables
- **Root cause:** `Stmt::Spawn` implementation creates a new isolated `Interpreter::new()` with no shared environment
- **Location:** `src/interpreter/mod.rs:2585-2596`
- **Code:**
  ```rust
  Stmt::Spawn { body } => {
      let body_clone = body.clone();
      std::thread::spawn(move || {
          let mut thread_interp = Interpreter::new();  // <- isolated!
          thread_interp.eval_stmts(&body_clone);
      });
  }
  ```
- **Implication:** Spawn is "fire and forget" only. Cannot share variables or channels with parent scope.
- **Workaround:** For concurrent work with channels, use async patterns: `parallel_map`, `par_each`, or Promise-based concurrency
- **Future work:** To make spawn truly useful with channels, would need:
  - Thread-safe shared environment (Arc<Mutex<Environment>>)
  - Or message-passing only architecture (pass channels as arguments to spawn)

### Gotcha 5: String Concatenation Doesn't Auto-Convert Numbers

- **Symptom:** `print("Value: " + 42)` fails with "Type mismatch in binary operation"
- **Root cause:** String + operator only works with String + String. No automatic int-to-string conversion.
- **Workaround:** Use `to_string()` function: `"Value: " + to_string(42)` OR use separate print statements
- **Prevention:** Examples should not assume auto-conversion. Document that Kujo requires explicit conversion.

### Gotcha 6: Null Comparison Uses Type Check, Not Keyword

- **Symptom:** `if result == null` fails with "Undefined global: null"
- **Root cause:** There's no `null` keyword in Kujo. The null value exists but isn't accessible via keyword.
- **Correct pattern:** Use `type(result) == "null"` to check for null values
- **Prevention:** Document that null checking uses type() function, not == null syntax

---

## Things I Learned

### Mental Model: Dual Execution Paths (Interpreter vs VM)

Kujo has TWO separate execution paths that must both handle features:

1. **Interpreter path** (`src/interpreter/mod.rs`):
   - Tree-walking evaluation
   - Used for non-hot code
   - Handles method calls via `Call` expression with `FieldAccess` function detection
   
2. **VM/JIT path** (`src/vm.rs`):
   - Bytecode execution
   - Used for hot functions (after 100 calls or 1 call for loops)
   - Handles method calls via separate `FieldGet` then `Call` opcodes

**Rule:** When adding new Value types with methods, implement support in BOTH paths:
- Interpreter: Add case in `eval_expr(Expr::Call { function, args })` where function is `FieldAccess`
- VM: Add case in `OpCode::FieldGet` handler AND update `call_native_function_vm` for special method prefixes

### Rule: Channel Methods Are Method Markers, Not Real Functions

The `Value::NativeFunction("__channel_method_send")` isn't a real function you can call.
It's a marker that tells the VM "the object is on the stack, and you need to call this method on it."

This pattern could be generalized for other non-struct types with methods (HttpServer, Image, etc.)

### Invariant: FieldGet Must Leave Stack in Valid State for Call

When FieldGet returns a method marker, the stack must be arranged so Call can work:
```
Before FieldGet: [... | channel_obj]
After FieldGet:  [... | channel_obj | method_marker]
After LoadArg:   [... | channel_obj | method_marker | arg]
During Call:     pops arg, pops method_marker, pops channel_obj, executes method
```

The channel object must stay on stack because Call needs it.

### Design Note: Why Not Make Methods Real Functions?

Could we make `chan.send` return a real closure that captures the channel? 
**No**, because:
- Closures in Kujo are `Value::Function` with captured environment
- That's heavy for simple method access
- The method marker pattern is more efficient
- VM can optimize away the marker entirely in future

---

## Debug Notes

### Initial Error Investigation

- **Failing example:** `examples/concurrency_channels.kujo` line 12: `chan.send(42)`
- **Error:** `Runtime Error: Cannot access field on non-struct`
- **Repro steps:**
  ```bash
  cargo run --quiet -- run examples/concurrency_channels.kujo
  ```
- **First hypothesis:** Channel methods not implemented (WRONG)
  - Searched for channel implementation, found it exists in interpreter
  - Searched for `.send()` handling, found it in `eval_expr` for Call with FieldAccess
  
- **Second hypothesis:** JIT/VM path doesn't support channel methods (CORRECT)
  - Created minimal test: `chan := channel(); chan.send(42)`
  - Confirmed error still occurred
  - Searched for VM opcode handling, found `FieldGet` case
  - Confirmed Channel was not in the match arms

### Testing Progression

1. Fixed FieldGet to handle Channel ✓
2. Added method marker pattern ✓
3. Updated call_native_function_vm to handle markers ✓
4. Tested channel creation: works ✓
5. Tested channel send: works ✓
6. Tested channel receive: works ✓
7. Full example: string concatenation errors (separate issue)
8. Fixed string concatenation in examples ✓
9. Fixed null comparison in examples ✓
10. All examples pass ✓

---

## Follow-ups / TODO (For Future Agents)

- [ ] Consider generalizing the method marker pattern for HttpServer, Image, and other non-struct types with methods
- [ ] Improve spawn to support shared state (requires thread-safe Environment design)
- [ ] Add auto-conversion for string concatenation (controversial: may break type safety)
- [ ] Consider adding `null` keyword for cleaner null checks
- [ ] Add unit tests in `src/vm.rs` for channel method calls via bytecode
- [ ] Document the dual execution path (interpreter vs VM) in ARCHITECTURE.md

---

## Links / References

### Files touched:
- `src/vm.rs` (FieldGet opcode, call_native_function_vm)
- `src/interpreter/mod.rs` (reviewed existing channel method handling)
- `examples/concurrency_channels.kujo` (fixed examples)
- `CHANGELOG.md` (documented fix)
- `ROADMAP.md` (updated status)
- `README.md` (updated examples)

### Related docs:
- `ROADMAP.md` - Concurrency section (lines 200-250)
- `README.md` - Concurrency & Parallelism section (lines 1290-1345)
- `.github/AGENT_INSTRUCTIONS.md` - Commit conventions and workflow

### Related existing implementations:
- Interpreter channel handling: `src/interpreter/mod.rs:3267-3309`
- Channel Value type: `src/interpreter/value.rs:333`
- Channel creation: `src/interpreter/native_functions/concurrency.rs:10-16`
- Spawn implementation: `src/interpreter/mod.rs:2585-2596`
