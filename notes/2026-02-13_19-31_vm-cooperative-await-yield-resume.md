# Kujo Field Notes — VM cooperative await yield/resume and context switching

**Date:** 2026-02-13
**Session:** 19:31 local
**Branch/Commit:** main / 1f73670
**Scope:** Implemented the P0 async VM path from roadmap Option 1 by adding suspendable VM context switching plus cooperative `Await` yield/resume behavior, with full tests and docs updates.

---

## What I Changed
- Added VM execution snapshot/restore APIs in `src/vm.rs`:
  - `save_execution_state()`
  - `restore_execution_state(snapshot)`
- Added VM execution context lifecycle APIs in `src/vm.rs`:
  - `create_execution_context(snapshot)`
  - `create_execution_context_from_current()`
  - `switch_execution_context(context_id)`
  - `remove_execution_context(context_id)`
  - `list_execution_context_ids()` / `active_execution_context_id()` / `has_execution_context(...)`
- Added cooperative execution APIs in `src/vm.rs`:
  - `execute_until_suspend(chunk)`
  - `resume_execution_context(context_id)`
  - `VmExecutionResult::{Completed, Suspended}`
- Changed `OpCode::Await` in `src/vm.rs` to support cooperative non-blocking behavior via `try_recv()` in cooperative mode.
- Added VM tests in `src/vm.rs` for:
  - snapshot round-trip + mutation isolation + globals replacement
  - context creation/switch/remove semantics
  - cooperative await suspension/resume and cooperative completion
- Updated `CHANGELOG.md`, `ROADMAP.md`, and `README.md` to record completion status.

## Gotchas (Read This Next Time)
- **Gotcha:** Cooperative VM suspension currently propagates through an internal sentinel error string.
  - **Symptom:** `execute()` still returns `Result<Value, String>`, so suspension cannot be returned as a native enum from the core loop.
  - **Root cause:** Existing VM execution contract is error-string based for non-success paths.
  - **Fix:** Introduced `VM_SUSPEND_ERROR_PREFIX` + parse helpers and wrapped this in public cooperative APIs returning `VmExecutionResult`.
  - **Prevention:** Do not expose the sentinel outside VM internals; call `execute_until_suspend` / `resume_execution_context` instead of parsing errors externally.

- **Gotcha:** Suspended `Await` must restore both promise and instruction pointer before snapshotting.
  - **Symptom:** On resume, execution can skip `Await` or lose promise state if only snapshot is taken.
  - **Root cause:** The dispatch loop pre-increments IP and pops the promise before handling await logic.
  - **Fix:** For pending promises in cooperative mode, push promise back to stack and set `self.ip = self.ip.saturating_sub(1)` before saving context.
  - **Prevention:** Treat suspend points as "rewind-to-replay" boundaries whenever opcode handlers mutate stack/IP before completion.

- **Gotcha:** VM unit tests that invoke native functions directly need explicit global symbol registration.
  - **Symptom:** `Undefined global: async_sleep` in VM tests even though runtime has async support.
  - **Root cause:** Test helper path compiles/runs bytecode directly without registering every native function into VM globals for that test context.
  - **Fix:** Define needed native symbols in test globals (e.g., `globals.define("async_sleep", Value::NativeFunction("async_sleep"))`).
  - **Prevention:** In VM-level tests, register every native function name referenced by compiled Kujo code.

## Things I Learned
- The async VM transition is safest when introduced as additive APIs (`execute_until_suspend`, `resume_execution_context`) instead of changing `execute()` signature immediately.
- Context switching works best as snapshot replacement semantics (save active context first, then restore target) so scheduler logic can stay simple.
- **Rule:** Any cooperative suspend point in the opcode loop must preserve replay correctness (IP + stack + mutable runtime metadata) before returning control.

## Debug Notes (Only if applicable)
- **Failing test / error:** `cooperative execution should not error: "Undefined global: async_sleep"`
- **Repro steps:** `cargo test vm::tests::test_execute_until_suspend_and_resume_for_pending_await`
- **Breakpoints / logs used:** Focused on VM test setup around globals registration and Await symbol resolution path.
- **Final diagnosis:** VM test context lacked explicit `async_sleep` binding in globals; adding native symbol fixed resolution.

## Follow-ups / TODO (For Future Agents)
- [ ] Replace sentinel suspension transport with a first-class internal execution state return type to remove string-encoded suspend signaling.
- [ ] Implement VM scheduler loop that drives multiple suspended contexts and polls/resumes fairly.
- [ ] Add scheduler-level integration test that interleaves at least 2 suspended contexts.

## Links / References
- Files touched:
  - `src/vm.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
