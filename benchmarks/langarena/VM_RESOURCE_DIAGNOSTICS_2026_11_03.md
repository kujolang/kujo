# VM resource diagnostics: matched baseline and candidate

The separate, unaccepted VM code-lifetime candidate passes three targeted tests
and the full current-source library suite (1,319 passed, eight existing ignores).
It also passes all 150 test-size benchmark checks, the matched generator suites
and 32 targeted callback/task integration tests. The no-JIT feature gate now
passes 178 tests; extended resource checks also pass. Production-size and platform acceptance remain open.

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
the system allocator to return pages to the OS. The extended resource gate below
now passes; production-size and platform checks remain before runtime acceptance.

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
certification. The memory patch remains separate and unaccepted. The remaining
production/platform gates must finish before integration.

## Generator lifetime regression

The complete generator integration suite passes **28/28 tests on both matched
baseline and candidate libraries**, built with explicit `runtime-jit` test
configuration. The new regression retains a generator, its environment and a
bytecode helper after dropping the original VM. It checks all four combinations
of creator/resumer JIT enablement, resumes yields and an explicit exception
handler, checks exhaustion and scope restoration, and calls the retained helper
before destruction and after transfer to the new VM.

Generator continuation bodies deliberately disable JIT. The test therefore also
compiles and invokes the helper outside continuation dispatch, and verifies that
resumption restores the caller's JIT mode. The JIT-disabled resumer cases avoid
allocating replacement native code that could mask a dangling entry. This is a
preservation gate, not a benchmark or proof of every escaping-value lifetime.

The first fixture incorrectly expected an undefined-variable load to enter
`except`; the baseline returned `Undefined variable: missing` (27 passed, one
failed). The corrected fixture uses explicit `throw`, matching the existing
generator handler contract. The initial source/failure, corrected source, both
matched successful suites, build commands, library/binary hashes and verifiers
are preserved in `vm-memory-generator-2026-11-03.json.gz` (15 hash-verified
artifacts). No runtime behavior was changed to make this test pass.

## Callback and task lifecycle gate

Nine unchanged integration suites pass **32/32 tests**, with no ignored tests:
`imported_vm_callback`, `callback_module_cache`, `jit_callback_depth`,
`language_task_contracts`, `jit_cold_callback`, `task_jit_policy`,
`task_module_snapshot_contracts`, `task_global_snapshot_contracts` and
`imported_task_globals_contracts`.

One optimized test harness links the frozen candidate library; subprocess tests
use the same frozen candidate CLI as the all-150 gate. Explicit `runtime-jit`
configuration retains native-worker and cancellation controls. The suites cover
VM/interpreter/JIT callback behavior, import cache recovery, captured values,
task-global snapshots, capability denial, cancellation and task admission after
cancellation. Existing deadlines and native-participation assertions are
unchanged. The runner verifies each suite's presence and every listed test's
successful result, plus source/library/CLI identities before and after execution.

Independent verification confirms all 32 test identities, nine suite identities,
zero failed/ignored tests and 19 hash-verified artifacts in
`vm-memory-lifecycle-2026-11-03.json.gz`. This completes the selected lifecycle
integration gate; it does not claim universal concurrency or platform coverage.
The no-JIT build/parity gate subsequently passed, as recorded below, using CI's
`runtime-network,runtime-archive,runtime-image` feature set in release mode with
the generator regression added to its four existing parity suites.

## No-JIT feature acceptance

The separate candidate passes **178 tests across five suites**, with no failures
or ignored tests, using `--no-default-features --features
runtime-network,runtime-archive,runtime-image` in the release profile. This is
the CI optional-feature set plus the generator suite. The build completed in
57m49s and Cargo exited zero.

| Suite | Passed |
| --- | ---: |
| Generator continuations | 28 |
| Native representation matrix | 6 |
| Native representation parity | 18 |
| Token estimation release contract | 2 |
| VM/interpreter parity surfaces | 124 |

Independent verification matches every passing test name to its exact binary's
`--list` output, verifies all 154 source/manifest identities and records
the five executable hashes. `vm-memory-no-jit-2026-11-03.json.gz` preserves
168 verified artifacts, including source snapshots, logs, manifests, test
membership and verifier. This proves feature compatibility, not JIT reclamation
in a build where JIT is disabled. The runtime candidate remains separate.

The first extended resource-probe build failed because the Apple linker could
not parse Rust LLVM22 bitcode with its LLVM17 reader. Its failure is preserved
in local `long-probes/` evidence. The successful matched rebuild uses Rust-managed
thin LTO in `long-probes-v2/`; its extended resource results are recorded below.

## Extended same-process lifetime checks

The rebuilt matched probes pass **60 process outcomes** at 256 and 1,024 owners,
with three repetitions for each count and mode. The independent verifier checks
**38,400 lifetime observations**: 30,720 VM destructions and 7,680 public-compiler
controls. It verifies exact process coverage, source/binary/raw-output identities
and all 12 rejected invalid-count controls (0 and 1,025 for three probe modes
and both binaries). Baseline retention fails the candidate zero-growth predicate
in all 12 applicable negative-control groups.

| Destroyed JIT VMs | Baseline requested-live and executable growth | Candidate growth |
| --- | ---: | ---: |
| 256 | 1,048,576 bytes | 0 |
| 1,024 | 4,194,304 bytes | 0 |

Every intermediate drop matches the expected growth, in every repetition. VM-only
controls remain flat; public compiler destruction still retains 256/1,024 distinct
code pages, preserving its existing API lifetime. No stale code is invoked.
These are instrumented allocation and mapping counts, not RSS, elapsed time or
a claim about all program shapes. Production-size and platform gates remain open.

The three probe count guards are extended from 64 to 1,024. Both frozen default-JIT
libraries are linked with identical Rust-managed thin-LTO flags; library and
probe hashes are recorded. All 12 boundary controls reject the expected counts
with the exact guard assertion. The initial old-guard attempt and linker failure
remain preserved, and are not counted as runtime failures or successful controls.

`vm-memory-extended-resources-2026-11-03.json.gz` preserves 174 verified
artifacts: both build attempts, raw matrices, identities, source patch, collectors,
verifier and source probes. Executables and libraries are identified by hash, not
embedded. The separate runtime patch is still unaccepted and unmerged.
