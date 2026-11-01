# SSG harness executable-start race

Linux release-mode CI run `41263681636`, job `124721922131`, failed the library
test `test_run_ssg_benchmark_profile_toggle_controls_stage_profile_capture`:
the freshly written `kujo_profile_stub.sh` could not start with `Text file busy
(os error 26)`. The same library suite passed the preceding candidate. This is
a subprocess-start failure, not a successful benchmark result or JIT assertion.

The SSG harness previously returned every `Command::output` start error
immediately. Linux can retain a writable file descriptor in a concurrent fork
until that child executes, temporarily preventing another execution of that
file. Kujo's upgrader already handles this mechanism explicitly.

The candidate retries only Linux `ETXTBSY` start errors, for at most 250 ms,
with bounded 10 ms sleeps. A started child is never retried: its exit status,
stdout and stderr still determine the result. Other spawn errors and persistent
busy files remain failures. No workload, iteration count, checksum or timing
threshold changes. This is harness robustness, not a claimed runtime speedup.

The Linux regression deliberately holds a writable handle and first confirms
the OS error. It then checks persistent failure without execution, successful
single execution after the handle closes, and propagation of a real child exit
7 with exactly one recorded invocation. Formatting passes locally. Linux test
execution and the full release gate remain pending; the local host is macOS.
