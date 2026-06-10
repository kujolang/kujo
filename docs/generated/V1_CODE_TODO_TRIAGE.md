# V1 Code TODO/FIXME/HACK Triage

Generated: 2026-06-09
Source root: `src`

| ID | File | Line | Marker | Summary | Severity | Owner | Target Release Bucket | Scope | Rationale |
| --- | --- | ---: | --- | --- | --- | --- | --- | --- | --- |
| V1TODO-001 | `src/benchmarks/profiler.rs` | 107 | TODO |     /// TODO: Integrate into Value allocation for automatic tracking | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-002 | `src/benchmarks/profiler.rs` | 117 | TODO |     /// TODO: Integrate into Value drop for automatic tracking | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-003 | `src/benchmarks/profiler.rs` | 17 | TODO |     /// TODO: Implement sampling-based profiling (currently event-based) | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-004 | `src/benchmarks/profiler.rs` | 247 | TODO |     /// TODO: Integrate into VM function call/return for automatic profiling | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-005 | `src/benchmarks/profiler.rs` | 256 | TODO |     /// TODO: Integrate into Value::new() for automatic tracking | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-006 | `src/benchmarks/profiler.rs` | 265 | TODO |     /// TODO: Integrate into Value::drop() for automatic tracking | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-007 | `src/benchmarks/profiler.rs` | 274 | TODO |     /// TODO: Call from JitCompiler::compile() | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-008 | `src/benchmarks/profiler.rs` | 284 | TODO |     /// TODO: Call from JitCompiler when recompiling due to guard failures | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-009 | `src/benchmarks/profiler.rs` | 293 | TODO |     /// TODO: Call from VM when JIT cache lookup succeeds | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-010 | `src/benchmarks/profiler.rs` | 302 | TODO |     /// TODO: Call from VM when JIT cache lookup fails | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-011 | `src/benchmarks/profiler.rs` | 311 | TODO |     /// TODO: Call from JIT-compiled code when type guards pass | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-012 | `src/benchmarks/profiler.rs` | 320 | TODO |     /// TODO: Call from JIT-compiled code when type guards fail | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-013 | `src/benchmarks/profiler.rs` | 52 | TODO |     /// TODO: Integrate into VM execution loop for automatic tracking | low | perf-owner | post-v1 | non-production | benchmark/profiler integration backlog outside production execution path |
| V1TODO-014 | `src/jit.rs` | 136 | TODO |     #[allow(dead_code)] // TODO: Will be used when JIT fully integrated into VM loop | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-015 | `src/jit.rs` | 199 | TODO |     #[allow(dead_code)] // TODO: Will be used when variable hashing is implemented | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-016 | `src/jit.rs` | 2927 | TODO |     /// TODO: Future optimization - keep frequently used variables in registers | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-017 | `src/jit.rs` | 304 | TODO | #[allow(dead_code)] // TODO: Integrate into VM execution loop for automatic profiling | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-018 | `src/jit.rs` | 366 | TODO | #[allow(dead_code)] // TODO: Used in adaptive recompilation decisions | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-019 | `src/jit.rs` | 377 | TODO | #[allow(dead_code)] // TODO: Integrate into VM hot path detection | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-020 | `src/jit.rs` | 4151 | TODO |                 // TODO: Implement proper tail-call optimization or save/restore slots | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-021 | `src/jit.rs` | 8090 | TODO |     /// TODO: Integrate into VM execution loop for automatic profiling | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-022 | `src/jit.rs` | 8107 | TODO |     /// TODO: Call from JIT-compiled code guard checks | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-023 | `src/jit.rs` | 8116 | TODO |     /// TODO: Call from JIT-compiled code when guard checks fail | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-024 | `src/jit.rs` | 8134 | TODO |     /// TODO: Use for adaptive recompilation decisions | low | jit-owner | post-v1 | production | experimental JIT backlog outside default release-critical runtime path |
| V1TODO-025 | `src/type_checker.rs` | 2439 | TODO |                 // TODO: Implement proper type checking for destructuring patterns | medium | typing-owner | post-v1 | production | optional typing/type-inference backlog outside runtime enforcement path |
| V1TODO-026 | `src/type_checker.rs` | 2915 | TODO |                 None // TODO: Return struct type when struct types are implemented | medium | typing-owner | post-v1 | production | optional typing/type-inference backlog outside runtime enforcement path |
| V1TODO-027 | `src/type_checker.rs` | 2921 | TODO |                 None // TODO: Look up field type from struct definition | medium | typing-owner | post-v1 | production | optional typing/type-inference backlog outside runtime enforcement path |
| V1TODO-028 | `src/type_checker.rs` | 3158 | TODO |                 // TODO: If we know it's a Promise<T>, return T | medium | typing-owner | post-v1 | production | optional typing/type-inference backlog outside runtime enforcement path |
| V1TODO-029 | `src/type_checker.rs` | 3233 | TODO |     /// TODO: This will be used when adding "Did you mean?" suggestions to interpreter | medium | typing-owner | post-v1 | production | optional typing/type-inference backlog outside runtime enforcement path |

Summary: `29` markers triaged, `0` unclassified.
