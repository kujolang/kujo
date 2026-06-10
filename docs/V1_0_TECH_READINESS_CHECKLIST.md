# Kujo v1.0 Tech Readiness Checklist

Status: active secondary checklist for pre-v1 technical follow-through.
Created: 2026-05-21
Source: `docs/V1_0_SENIOR_CODEBASE_AUDIT_2026-05-21.md`

Use this checklist only when all remaining unchecked items in `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` are blocked.

- [x] **V1TRIAGE-001**: Resolve `cargo fmt --check` drift and capture evidence.
  - Evidence (2026-05-21):
    - Resolved deterministic rustfmt drift via `cargo fmt`, then revalidated a clean gate with `cargo fmt --check`.
    - Captured command outcomes and touched-file summary in `notes/2026-05-21_10-45_v1triage-001_fmt_drift_evidence.md`.
- [x] **V1TRUN-001**: Finish channel receive/runtime TODO follow-through in active runtime paths.
  - Evidence (2026-05-21):
    - Implemented lock-safe blocking receive in active interpreter channel method-call paths and removed the production TODO.
    - Added channel receive/send regression coverage in `tests/interpreter_tests.rs` for blocking behavior, FIFO ordering, and arity failure handling.
    - Validation: `cargo test --test interpreter_tests channel_receive`, `cargo test --test vm_interpreter_parity_surfaces`, and `cargo test`.
- [x] **V1TRUN-002**: Finish iterator filter/map/generator-next TODO follow-through in active runtime paths.
  - Evidence (2026-05-21):
    - Implemented iterator-stage chaining for `filter`/`map`, nested-iterator materialization support, and generator-backed `next()` filter continuation behavior in active interpreter runtime paths.
    - Added runtime regressions in `tests/interpreter_tests.rs` for chained filter/map semantics and generator-next filtered-yield progression.
    - Regenerated triage artifacts (`docs/generated/V1_CODE_TODO_TRIAGE.md`/`.csv`) and updated triage contract assertions for the new TODO state.
    - Validation: `cargo test --test interpreter_tests iterator_`, `cargo test --test vm_interpreter_parity_surfaces`, `cargo test --test v1_code_todo_triage_contract`, and `cargo test`.
- [x] **V1TUNSAFE-001**: Create/refresh machine-verifiable unsafe inventory and safety classification, then start reducing executable unsafe sites with tests (no broad rewrites).
  - Evidence (2026-05-21):
    - Added reproducible unsafe inventory generator `scripts/generate_unsafe_inventory.sh` and contract tests in `tests/unsafe_inventory_contract.rs`.
    - Generated `docs/generated/UNSAFE_INVENTORY.md` and `docs/generated/UNSAFE_INVENTORY.csv` with strict zero-unknown classification validation.
    - Reduced executable unsafe callsites in `src/jit.rs` return-value optimization test flow by introducing the safe wrapper `set_return_int(&mut VMContext, i64)` and replacing repeated ad-hoc unsafe invocations.
    - Validation: unsafe inventory generation command, `cargo test --test unsafe_inventory_contract`, `cargo test test_return_value_optimization`, and `cargo test --test vm_interpreter_parity_surfaces`.
