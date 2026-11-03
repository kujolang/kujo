# VM resource diagnostics: candidate observations

The separate, unaccepted VM code-lifetime candidate passes three targeted tests
and the full current-source library suite (1,319 passed, eight existing ignores).
It has not completed integration, no-JIT, all-benchmark or platform acceptance.

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
behavior. A matched baseline is still required before claiming a measured fix.
Use identical source/toolchain/features/vendor paths and freeze the linked
artifact: copying a Kujo rlib alone does not freeze its transitive dependency
metadata. The baseline is being rebuilt in the same isolated path; the runtime
patch is restored after that build. Do not infer resource results on Windows,
where the pinned backend has a separate deallocation limitation.
