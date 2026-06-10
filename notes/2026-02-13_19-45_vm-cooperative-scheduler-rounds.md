# Kujo Field Notes — VM Cooperative Scheduler Rounds

**Date:** 2026-02-13
**Session:** 19:45 local
**Branch/Commit:** main / 993295b
**Scope:** Completed the next highest-priority incomplete roadmap item for async VM Option 1 by implementing cooperative scheduler APIs in the VM and expanding scheduler-specific test coverage. Also updated roadmap/changelog/readme status for this milestone.

---

## What I Changed
- Added cooperative scheduler APIs in `src/vm.rs`:
  - `pending_execution_context_count()`
  - `run_scheduler_round()`
  - `run_scheduler_until_complete(max_rounds)`
- Added scheduler round result type in `src/vm.rs`:
  - `VmSchedulerRoundResult { completed_contexts, pending_contexts }`
- Updated `resume_execution_context(context_id)` in `src/vm.rs` to remove completed contexts and clear `active_execution_context` when that context finishes.
- Added/expanded VM tests in `src/vm.rs` for:
  - completed-context cleanup after resume
  - scheduler round completion across multiple contexts
  - pending-round behavior for unresolved async sleep promises
  - scheduler validation (`max_rounds == 0`) and round-budget exhaustion errors
- Updated docs:
  - `CHANGELOG.md` (`[Unreleased]` scheduler entry)
  - `ROADMAP.md` (marked async VM scheduler step complete)
  - `README.md` (documented scheduler API milestone)

## Gotchas (Read This Next Time)
- **Gotcha:** Direct `BytecodeChunk { ... }` test initialization failed after struct changes.
  - **Symptom:** Rust compile error: `missing fields 'is_async', 'is_generator', 'local_count' and ... in initializer of 'bytecode::BytecodeChunk'`.
  - **Root cause:** `BytecodeChunk` gained additional fields; old struct-literal test setup no longer covered all required fields.
  - **Fix:** Use `BytecodeChunk::new()` in tests, then set only required fields (`name`, `instructions`) explicitly.
  - **Prevention:** Rule: For VM tests, construct chunks with `BytecodeChunk::new()` unless the test explicitly needs non-default metadata fields.

- **Gotcha:** Scheduler rounds can appear idle if promises are not ready yet.
  - **Symptom:** `run_scheduler_round()` may return `completed_contexts == 0` while contexts remain pending.
  - **Root cause:** Cooperative await path uses non-blocking `try_recv()`; unresolved promises legitimately suspend again.
  - **Fix:** `run_scheduler_until_complete()` includes bounded retry rounds and a short backoff sleep when no context completes.
  - **Prevention:** Treat zero-completion rounds as expected under pending I/O, not as scheduler failure.

## Things I Learned
- The async VM architecture now has a clean layering: snapshot/restore → context switching → cooperative await suspend/resume → scheduler driving pending contexts.
- Cleanup semantics belong in `resume_execution_context()` for correctness: if completion does not evict context state, scheduler bookkeeping drifts and stale contexts accumulate.
- For cooperative schedulers, deterministic context ordering (`list_execution_context_ids()` sorted) keeps behavior and tests stable.
- The right test shape for async scheduler behavior is eventual-completion loops with tiny sleeps; strict single-tick completion assumptions are flaky.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error[E0063]: missing fields 'is_async', 'is_generator', 'local_count' and 3 other fields in initializer of 'bytecode::BytecodeChunk'`.
- **Repro steps:** Run `cargo test vm::tests::test_run_scheduler -- --nocapture` after creating context snapshots with direct `BytecodeChunk { ... }` literals in VM tests.
- **Breakpoints / logs used:** Compile output and targeted scheduler test runs (`cargo test vm::tests::test_run_scheduler_ -- --nocapture`).
- **Final diagnosis:** VM tests used outdated `BytecodeChunk` literal construction; switching to `BytecodeChunk::new()` resolved build failures and made tests resilient to future default-field additions.

## Follow-ups / TODO (For Future Agents)
- [ ] Run SSG benchmark path for async VM scheduler milestone and record before/after timing in roadmap/changelog if target improves.
- [ ] Consider exposing scheduler round metrics through CLI/profile output for async runtime tuning.

## Links / References
- Files touched:
  - `src/vm.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
