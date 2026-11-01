# JIT code mapping lifetime diagnostic

Accepted runtime `782c712d`, macOS x64. The diagnostic links the frozen accepted
release library; source, library and executable identities are retained in
[jit-code-lifetime-2026-11-01.json](jit-code-lifetime-2026-11-01.json).
The [probe](probes/jit_code_lifetime.rs) uses `mach_vm_region` with containing-region
bounds, releases returned Mach port rights, and never calls or dereferences code
after its compiler is dropped.

Two fresh-process repetitions at each size passed both mapped/unmapped controls
and result accounting:

| Sequential compiler instances | Distinct code pages still mapped | Mapped-byte lower bound |
|---|---:|---:|
| 1 | 1 | 4,096 |
| 8 | 8 | 32,768 |
| 32 | 32 | 131,072 |

Each instance compiles a constant-return chunk, checks the live mapping, drops
the compiler and queries the mapping again. Both repetitions agree. These are
mapped-address lower bounds, not resident memory, complete allocation sizes,
production workload growth, VM lifecycle measurements or elapsed-time speedups.

Pinned Cranelift 0.113.1 deliberately forgets allocations in `Memory::drop` to
retain raw function pointers; `JITModule::free_memory` requires proof that no
code remains executing or callable. Kujo's public compiler exposes raw compiled
function pointers. Blanket reclamation in `JitCompiler::drop` is therefore not
justified by this experiment. A VM-owned compiler lifetime audit and dedicated
VM lifecycle reproduction are the next gates before any runtime change.

The initial `mincore` query was rejected: its unmapped negative control failed.
No retention conclusion uses that implementation. The corrected Mach query
passes the independent mmap/munmap control and all six compiler observations.

Verification: release probe build succeeded; six fresh-process runs exited zero
with empty stderr; controls and JSON accounting passed; `cargo fmt --check` and
standalone `rustfmt --check` passed. No runtime source changed. Existing all-50
checksum results are prior evidence, not rerun or claimed as new here.

Reproduction (macOS, use a release library whose source identity is recorded):

```sh
rustc --edition=2021 -O -C lto=thin benchmarks/langarena/probes/jit_code_lifetime.rs \
  --extern kujo=/path/to/recorded/libkujo.rlib -L dependency=target/release/deps \
  -o /tmp/jit-code-lifetime
/tmp/jit-code-lifetime 1
/tmp/jit-code-lifetime 8
/tmp/jit-code-lifetime 32
```
