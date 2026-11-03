# VM resource diagnostics: matched baseline and candidate

The separate, unaccepted VM code-lifetime candidate passes three targeted tests
and the full current-source library suite (1,319 passed, eight existing ignores).
It also passes all 150 test-size benchmark checks. It has not completed
integration, no-JIT, production-size or platform acceptance.

The new `probes/vm_resource_probe.rs` links three diagnostics to one recorded
Kujo library. Run it as `COUNT jit|vm live|vm-maps|compiler-maps` on macOS.
The existing mapping probes are included unchanged. Its live-allocation probe
forwards to Rust's system allocator and accounts for successful allocations,
deallocation and successful realloc size changes. Positive controls verify
zeroing, growth, shrink and final balance. A warmed lifecycle then compiles and
calls a function, checks the returned value, and drops both VM and function.
Numeric observations are preallocated; formatting occurs after measurement.

`probes/collect-vm-resources.rb RESOURCE_PROBE NEW_OUTPUT` runs 1/8/32 owners,
three repetitions each: VM and JIT live allocations, VM and JIT executable
mappings, and public JitCompiler mapping retention. All 45 candidate processes
exit successfully. Independent inspection of every raw observation confirms:

- Outstanding requested allocation bytes return to the warmed baseline after
  every VM destruction, both with JIT enabled and disabled.
- Executable virtual mapping bytes return to baseline after every VM destruction.
- Public compiler destruction retains 1/8/32 code pages, matching its deliberately
  unchanged lifetime behavior. The probe never calls or dereferences stale code.

Instrumented process peak RSS remains distinct: the JIT live-allocation runs
range from 6,377,472 to 7,327,744 bytes across counts/repetitions. Zero outstanding
allocation growth does not imply zero retained allocator pages or lower peak RSS.
These measurements are not elapsed-time performance results.

The candidate library and probe identities, source patch, raw observations and
verification receipts are retained in `vm-resource-candidate-2026-11-03.json.gz`.
The archive explicitly describes an unaccepted candidate, not main-branch runtime
behavior. The matched comparison below now establishes the scoped reclamation effect.
Use identical source/toolchain/features/vendor paths and freeze the linked
artifact: copying a Kujo rlib alone does not freeze its transitive dependency
metadata. The baseline was rebuilt in the same isolated path; the runtime
patch was restored and its source hashes verified after that build. Do not infer resource results on Windows,
where the pinned backend has a separate deallocation limitation.

## Matched result

All 90 process outcomes pass `probes/compare-vm-resources.rb`, which checks raw
outputs, exact matrix identities, binary hashes, RSS receipts and every VM-drop
observation. Passing candidate data supplied as both versions is rejected by the
negative control because it does not reproduce the baseline leak.

| Destroyed JIT VMs | Baseline live retained bytes | Candidate live retained bytes |
| --- | ---: | ---: |
| 1 | 4,096 | 0 |
| 8 | 32,768 | 0 |
| 32 | 131,072 | 0 |

Every repetition matches. Baseline executable mapping growth has the same byte
counts; candidate executable growth is zero. VM-only controls stay flat, and
public compiler retention stays unchanged. The 582 dependency artifact records
have matching Cargo metadata; all were reused for the baseline. Root crate build
metadata/features match, and the tested source difference is the isolated memory
patch. This metadata comparison is not a transitive binary checksum manifest.

Some candidate RSS observations are higher. For example, the 32-VM JIT live
probe has median peak RSS 6,828,032 bytes versus baseline 6,680,576 bytes. We do
not claim an RSS or speed improvement, nor that requested-byte reclamation forces
the system allocator to return pages to the OS. Longer resource, integration,
no-JIT, production-size and platform checks remain before runtime acceptance.

`vm-resource-comparison-2026-11-03.json.gz` preserves both raw matrices, the
matched-library metadata, comparison verifier and negative-control evidence.

## Current-source CLI and benchmark gate

The isolated candidate CLI build completed successfully in 23m29s. The exact
Cargo executable artifact was copied and hashed as
`17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`.
All 175 staged source/input/driver identities matched before the run; the
generated port matched its ordered source fragments byte-for-byte.

The unchanged 50 test-size inputs pass in VM, JIT and interpreter modes:
**150 passed, zero failed**. The sequential runner checks each configuration,
exit status, complete stdout checksum, summary line and truncation status; it
rechecks frozen identities before and after each mode. Independent inspection
repeated the coverage/checksum checks and verified all 14 archived artifacts in
`vm-memory-checksums-2026-11-03.json.gz`.

These are correctness results, not a timing comparison or production-size
certification. The memory patch remains separate and unaccepted. Matched
generator-lifetime regression execution and the remaining integration/no-JIT/
platform gates must finish before integration.
