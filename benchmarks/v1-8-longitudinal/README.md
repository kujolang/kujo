# Kujo longitudinal release workloads

These deterministic, offline programs are the stable workload cohort introduced
for the Kujo 1.8 release-readiness tranche. They are intended to remain useful
for later releases rather than support a one-off headline.

| Workload | Representative surface |
| --- | --- |
| `data_cli.kujo` | collection filtering, transformation, aggregation and JSON serialization |
| `project/main.kujo` | multi-file imports, exported callables/values and project checking |
| `generator_heavy.kujo` | nested generators, repeated suspension/resumption and shared aliases |
| `async_concurrent.kujo` | tasks, reusable completion promises, channels and wakeups |
| `mixed_application.kujo` | structs, methods, struct generators, closures, loops and formatting |

Correctness, VM/interpreter parity and root bytecode counts are covered by:

```bash
cargo test --test v1_8_longitudinal_workloads -- --nocapture
```

Release timing campaigns must record immutable binary hashes, source commits,
toolchain, platform, build mode, warmups, repetitions and raw samples. Compare
interleaved baseline/candidate runs and treat deltas inside observed variance as
inconclusive. Allocator totals remain unsupported unless the host supplies a
reliable attribution tool.
