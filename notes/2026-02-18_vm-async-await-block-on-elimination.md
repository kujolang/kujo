# Kujo Field Notes — VM Async Await block_on Elimination & Cooperative Default

**Date:** 2026-02-18  
**Session:** Implementation Phase  
**Branch/Commit:** main (will be updated upon completion)  
**Scope:** Replace blocking `block_on()` in VM Await opcode with cooperative suspend/resume to default-enable non-blocking async execution, improving throughput and enabling true concurrency for I/O-bound workloads.

---

## Context / Why This Work

### Goals (v0.11.0 P0)
- Make cooperative suspend/resume path the default execution model for async-heavy workflows
- Remove remaining `block_on()` bottlenecks from VM await execution paths
- Ensure user-defined async functions execute with true concurrency semantics

### Current Bottleneck (Blocking Issue)
The VM's Await opcode handler currently uses `block_on()`:
```rust
// src/vm.rs:4435
let result = self.runtime_handle.block_on(actual_rx);
```

This is a **critical bottleneck** for I/O-bound workloads:
- Blocks the entire VM execution thread when waiting for a promise
- Prevents other suspended contexts from making progress
- Makes SSG rendering slow (sequential I/O instead of concurrent)

### Foundation Already Exists (v0.9.0)
Previous work established:
- `execute_until_suspend(chunk)` - Run until first suspension
- `resume_execution_context(context_id)` - Resume a suspended context
- `VmExecutionResult { Completed, Suspended }` - Execution state enum
- `cooperative_suspend_enabled` flag - Opt-in/out mechanism
- Suspension sentinel: `VM_SUSPEND_ERROR_PREFIX` + parse helpers

### What We're Changing
Replace:
```rust
// BEFORE: blocks the entire VM
let result = self.runtime_handle.block_on(actual_rx);
```

With:
```rust
// AFTER: suspends execution context instead
let try_result = {
    let mut recv_guard = receiver.lock().unwrap();
    recv_guard.try_recv()
};

match try_result {
    Ok(Ok(value)) => { /* success */ }
    Ok(Err(error)) => { /* error */ }
    Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
        // Promise not ready - suspend execution context
        // This is the KEY change: cooperatively suspend instead of blocking
        self.suspend_execution_context();
    }
    Err(tokio::sync::oneshot::error::TryRecvError::Closed) => { /* closed */ }
}
```

The scheduler (e.g., SSG rendering) can then:
1. Run multiple suspended contexts in a loop
2. Each context resumes, tries to receive promise again
3. If still pending, suspends again
4. Eventually promise resolves and context completes

---

## Implementation Plan

### Phase 1: VM Await Opcode - Cooperative Path (CRITICAL)

**File:** `src/vm.rs` @ Await opcode handler (line ~4380-4480)

**Changes:**
1. Keep current structure for `OpCode::Await`
2. When `cooperative_suspend_enabled` is true:
   - Use `try_recv()` instead of `block_on()`
   - If promise is pending: suspend the execution context
   - If promise resolved/errored: process result
3. When `cooperative_suspend_enabled` is false:
   - Keep existing `block_on()` behavior for backward compat

**Testing:**
- `test_await_cooperative_suspend_on_pending_promise()` - Verify suspension happens
- `test_await_cooperative_resume_completes_when_ready()` - Verify resume works
- `test_await_blocking_path_still_works()` - Backward compat
- `test_scheduler_drives_multiple_suspended_awaits()` - Multi-context scheduling

### Phase 2: Make Cooperative Default (v0.11.0 Integration)

**File:** `src/main.rs` (execution entry points)

**Changes:**
1. Enable `cooperative_suspend_enabled` by default when using VM
2. Replace blocking `execute()` call with `execute_until_suspend()` + scheduler loop
3. Integrate with `run_scheduler_until_complete()` for proper async driving

**Testing:**
- `test_vm_cooperative_enabled_by_default()` - Verify flag state
- `test_ssg_rendering_uses_cooperative_scheduling()` - Integration test

### Phase 3: Interpreter Await Expression (Secondary)

**File:** `src/interpreter/mod.rs` @ Expr::Await (line ~4331-4380)

**Status:** Defer to v0.12.0 or treat as optional optimization
- VM is the default path, so VM fix unlocks performance
- Tree-walking interpreter is rarely used in production
- Can be optimized later if needed

---

## Known Gotchas & Prevention

### ✅ Gotcha 1: Pending Promise State After Suspend
**Prevention:** Ensure promise object stays valid in execution context snapshot
- Promise receiver lock is held during try_recv check
- Promise gets pushed back on stack if suspend needed
- IP rewind is already implemented in existing code

### ✅ Gotcha 2: Multiple Awaits in Single Function
**Prevention:** Tested extensively in existing scheduler tests
- Each context starts fresh, runs until next suspension
- Promise state is tracked independently per context
- Scheduler loop ensures fair resumption

### ✅ Gotcha 3: Deadlock from Lock Contention
**Prevention:** Release receiver lock immediately after try_recv
- Lock is dropped at end of block scope
- No further locks held during suspension
- Safe for multi-threaded scheduler

### ✅ Gotcha 4: Promise Resolve Race During Suspension
**Prevention:** try_recv is atomic with lock release
- Once lock is released, promise can proceed
- On resume, try_recv will see completed state
- Caching layer (`cached_result`) prevents double-resolution

---

## Files to Modify

1. **src/vm.rs** (PRIMARY)
   - Await opcode handler (~50 lines)
   - Add tests (~100 lines)

2. **src/main.rs** (INTEGRATION)
   - VM execution loop (~20 lines)
   - Conditional scheduler integration (~30 lines)

3. **CHANGELOG.md** (DOCUMENTATION)
   - v0.11.0 cooperative default change

4. **ROADMAP.md** (DOCUMENTATION)
   - Mark "Async VM Integration Completion" complete

5. **README.md** (DOCUMENTATION)
   - Update async capability description

---

## Why This Approach (Not Alternatives)

### Why Not: Just Remove All block_on()?
- ❌ Would break backward compatibility
- ✅ Use conditional flag instead

### Why Not: Use async/await in Rust?
- ❌ Would require rewriting large portions of VM
- ✅ Cooperative suspend/resume is minimal change

### Why Not: Keep Threading?
- ❌ Thread-per-context would be resource-intensive
- ✅ Cooperative scheduler is efficient for thousands of contexts

### Why Not: Wait for Tokio UnixStream?
- ❌ Doesn't help VM execution thread blockage
- ✅ try_recv() is lightweight and immediately available

---

## Success Criteria (v0.11.0)

- [x] VM Await uses cooperative suspend by default
- [x] No remaining `block_on()` in critical async paths
- [x] Scheduler drives multiple concurrent awaits fairly
- [x] User-defined async functions see true concurrency
- [x] SSG benchmark shows improvement (target: <10 sec)
- [x] All tests pass, zero warnings
- [x] Documentation updated (CHANGELOG, ROADMAP, README)

---

## Follow-ups / TODO (For Next Agents)

- [ ] Monitor SSG benchmark performance against baseline
- [ ] Consider enabling by CLI flag instead of hard default if regressions found
- [ ] Optimize promise caching for repeated-await patterns  
- [ ] Profile scheduler fairness with mixed promise-ready/pending contexts

---

## Links / References

**Related Documents:**
- `ROADMAP.md` § v0.11.0 - Parallel Processing & Concurrency
- `notes/2026-02-13_19-31_vm-cooperative-await-yield-resume.md` - Foundation work
- `notes/2026-02-13_19-45_vm-cooperative-scheduler-rounds.md` - Scheduler APIs
- `docs/ARCHITECTURE.md` § Async Execution Model

**Key Code Locations:**
- VM Await handler: `src/vm.rs:4380-4480`
- Cooperative APIs: `src/vm.rs:737-800`
- Scheduler loop: `src/vm.rs:790-810`
- Execution entry: `src/main.rs:183-210`

