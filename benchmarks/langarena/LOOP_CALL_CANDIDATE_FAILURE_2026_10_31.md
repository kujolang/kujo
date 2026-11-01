# Loop call candidate: rejected verification round

Candidate `4b94130178393b81d57b1bda728d4e203a851ed7` is **not accepted**.
The [release-mode regression job](https://github.com/kujolang/kujo/actions/runs/41261332562/job/124715058393)
compiled successfully but failed three existing native-entry assertions:

- `loop_array_literals_preserve_types_after_native_entry`
- `loop_call_resumes_without_replaying_arguments_or_prior_effects`
- `loop_callback_error_unwinds_after_native_resume`

Their value/output assertions passed; JIT diagnostics lacked the required native
loop entry. Correct VM fallback is insufficient for this optimization's acceptance.
The collection suite passed its other 12 tests. The library passed 1,309 tests
with eight ignored; four preceding integration suites passed seven tests total.
Cargo stopped before the new condition-call suite and later suites, so those
tests have no result in this run.

Source inspection identified an omitted dependency: loop compilation initialized
the scalar helpers but never set `call_func`. The new guarded Call continuation
therefore reaches translation that returns an error when this helper is absent.
Whole-function compilation already supplied it. Follow-up candidate `08d4b63`
registers the existing call helper with the shared helper declarations and adds
a direct loop-compilation regression. Its correctness and performance remain
unverified. No baseline or production benchmark implementation changed.

The next remote test command uses `--no-fail-fast` to collect all nine selected
suite results; any failed test still fails the job. The previous queued local
build was stopped before editing, and its manifests were archived. A replacement
continuation uses refreshed source hashes and still waits for the original
production run; the production run was not restarted.

The [compressed evidence](loop-call-candidate-failure-2026-10-31.json.gz) contains
four artifacts: run metadata, job metadata, exact source/toolchain receipt and
complete Cargo output. Every entry has a SHA-256 and base64 payload; decoding and
digest verification passed. This report preserves a failed experiment, not an
accepted speedup or a completed performance gate.
