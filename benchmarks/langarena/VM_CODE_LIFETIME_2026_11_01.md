# VM code lifetime baseline

Accepted runtime `782c712d`, macOS x64. Twelve fresh processes (two repeats of
1/8/32 VM instances, JIT enabled and disabled) pass mapping controls and explicit
compilation-result assertions. The [probe](probes/vm_code_lifetime.rs) calls the
public VM API to compile one constant-return function per instance, then destroys
the VM. No private pointer access or stale-code invocation is involved.

| VM instances | JIT executable mapping growth after destruction | VM-only growth |
|---|---:|---:|
| 1 | 4,096 bytes | 0 |
| 8 | 32,768 bytes | 0 |
| 32 | 131,072 bytes | 0 |

Both repetitions agree; every JIT observation retains the executable mapping
size seen while that VM was alive. These are whole-process executable virtual
mapping totals, not RSS or a complete accounting of code/data allocations.
No elapsed-time claim follows, and this small forced-compilation workload does
not measure production benchmark resource growth.

[Raw observations and artifact identities](vm-code-lifetime-2026-11-01.json)
include every live/post-drop measurement, frozen-library/source hashes and binary
hash. The accepted library is the same one used in the prior compiler-lifetime
probe. Build, all twelve runs and standalone/repository formatting passed.
This confirms the VM lifecycle issue separately from the public compiler API.

The [ownership audit](VM_CODE_OWNERSHIP_2026_11_01.md) supports investigating
VM-only reclamation. Public compiler destruction must retain its existing
behavior. No runtime fix is accepted in this report. The isolated candidate's
mapping regression must fail on the unchanged runtime before its proposed
reclamation method and VM destructor are tested. Returned functions, Values,
snapshots, JIT toggling, callbacks and existing parity gates must remain valid.

Reproduce using the same recorded library as the compiler probe:

```sh
rustc --edition=2021 -O -C lto=thin benchmarks/langarena/probes/vm_code_lifetime.rs \
  --extern kujo=/path/to/recorded/libkujo.rlib -L dependency=target/release/deps \
  -o /tmp/vm-code-lifetime
/tmp/vm-code-lifetime 32 vm
/tmp/vm-code-lifetime 32 jit
```

## Baseline regression gate

The release library gate finished with two semantic controls passing and exactly
one intended failure: `vm_drop_unmaps_its_compiled_entry` reports “VM-owned code
mapping must be reclaimed.” Exit status is 101. The mapping test runs in an
isolated child process so unrelated parallel tests cannot reuse a freed address.
The JIT-toggle and surviving-value/function/snapshot controls pass on the baseline.
The accepted runtime prefix is byte-identical; only the regression module was
appended. The [three-artifact baseline bundle](vm-code-lifetime-baseline-tests-2026-11-01.json.gz)
preserves the regression source, full log and source/executable identities.

The VM-only reclamation candidate is now applied in an isolated worktree, with
an additional assertion that the surviving function is recovered from restored
snapshot globals. Targeted and full-library gates are pending; this report does
not accept or integrate that runtime change.
