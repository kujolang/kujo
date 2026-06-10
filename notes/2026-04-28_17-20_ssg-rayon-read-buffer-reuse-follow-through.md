# Kujo Field Notes — SSG Rayon Read-Buffer Reuse Follow-Through

**Date:** 2026-04-28
**Session:** 17:20 local
**Branch/Commit:** main / b520c55
**Scope:** Implemented the next v0.11 P0 throughput follow-through by reusing per-worker read buffers in the Rayon SSG hot path, added focused regressions, and updated release/docs artifacts.

---

## What I Changed
- Optimized `ssg_run_rayon_read_render_write(...)` in `src/interpreter/native_functions/async_ops.rs` to use `rayon::map_init` with per-worker reusable `Vec<u8>` read buffers.
- Added `ssg_read_source_file_bytes(...)` helper and switched the Rayon read lane from per-file `std::fs::read(...)` allocation to clear-and-refill buffered reads.
- Added regression tests:
  - `test_ssg_read_source_file_bytes_reuses_buffer_across_large_then_small_reads`
  - `test_ssg_run_rayon_read_render_write_mixed_source_sizes_preserve_page_isolation`
- Updated docs:
  - `CHANGELOG.md` ([Unreleased] Added entry)
  - `ROADMAP.md` (completed milestone + remaining-workstream wording)
  - `README.md` (latest throughput-step bullet)

## Gotchas (Read This Next Time)
- **Gotcha:** Reused read buffers can silently retain stale bytes if the read path does not clear first.
  - **Symptom:** Small-file output can contain data from a prior larger file when reusing an in-memory buffer.
  - **Root cause:** Buffer reuse without explicit `clear()` before refill leaves previous-length assumptions invalid.
  - **Fix:** Centralized the read path in `ssg_read_source_file_bytes(...)` and always call `read_buffer.clear()` before `read_to_end(...)`.
  - **Prevention:** Keep a dedicated regression that reads large→small payloads with the same buffer and validates exact byte equality.

- **Gotcha:** Full-suite network round-trip tests may fail under restricted runtime permissions.
  - **Symptom:** `test_release_hardening_network_module_round_trip_behaviors` failed with `PermissionDenied` on ephemeral TCP bind.
  - **Root cause:** Environment/socket permission restriction, not SSG pipeline behavior.
  - **Fix:** None in code; treat as environment-specific test limitation.
  - **Prevention:** For SSG-only changes, rely on focused helper/pipeline suites plus full-suite attempt and report permission-bound failures explicitly.

## Things I Learned
- `rayon::map_init` is a clean way to carry per-worker scratch buffers through a parallel pipeline while preserving existing reduction/error contracts.
- The SSG helper contract surface (`checksum`, `files`, `read_ms`, `render_write_ms`, and read/write error message shape) is stable and should stay unchanged even for low-level throughput refactors.
- Isolation tests (mixed-size + marker-based non-bleed assertions) are high-signal for detecting buffer-reuse regressions.

## Debug Notes (Only if applicable)
- **Failing test / error:** `interpreter::native_functions::tests::test_release_hardening_network_module_round_trip_behaviors` failed with `Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }`.
- **Repro steps:** `cargo test` from repo root in current sandboxed environment.
- **Breakpoints / logs used:** N/A (failure is direct panic with clear bind error).
- **Final diagnosis:** Unrelated environment permission limit for ephemeral TCP bind.

## Follow-ups / TODO (For Future Agents)
- [ ] Benchmark `bench-ssg` median wall-clock impact of per-worker read-buffer reuse vs prior commit in comparable local conditions.
- [ ] Continue P0 throughput follow-through by targeting next residual render/write overhead without changing SSG output/metric contracts.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
