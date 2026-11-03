# VM-private code lifetime: scoped acceptance

**Decision: accept the tested VM-private ownership fix for integration.** Its
selected semantic, resource, feature, production and platform gates have passed.
This closes the separate candidate checkpoint, not the full performance campaign,
universal reclamation, all-150 production or a release.

Integrated `src`, `tests`, `Cargo.toml` and `Cargo.lock` match tested platform head
`089246278e955545d98415f66b851b01803a12ce` exactly. Platform commits add test/workflow
changes to candidate `75445306`; the production VM prefix and whole JIT source
remain identical to the measured candidate. Changelog and acceptance docs are
separate from tested code. Generated inventories are copied from that tested tree.

## Final targeted platform evidence

| Platform | Successful job | Tests | Executable revocation |
| --- | ---: | ---: | --- |
| Linux x64 | 125842289961 | 63 | Yes |
| Linux ARM64 | 125842289692 | 63 | Yes |
| macOS ARM64 | 125842289351 | 63 | Yes |
| macOS Intel | 125842289870 | 63 | Yes |
| Windows x64 | 125847779808 | 62 | Not proved |

Unix jobs are from `41613864999`, head `baaf23931e413474528e43f4cad0559bff75945b`.
Windows is from corrected run `41615482553`, head `089246278e955545d98415f66b851b01803a12ce`.
Those heads have identical runtime/test/Cargo source. The original Windows job
failed before tests; its failure is retained, not counted as a pass. Final macOS
Intel artifact `13225060672` and Windows artifact `13223813287` match their
run/job/source identities and exact three-file contents.

Unix runs each contain three owner tests plus sixty integrations across ten named
suites; Windows contains two owner tests plus the same sixty integrations. Exact
source-derived membership, successful builds, summaries and raw hashes verify;
no integration failures, ignores or filtering. Other library tests are deliberately
filtered. These are 314 executions, not 314 distinct definitions or five full
library runs.

## Acceptance evidence

| Requirement | Evidence |
| --- | --- |
| Ownership/safety | Consuming crate-private compiler release at VM destruction; six added executable unsafe inventory sites explicitly reviewed; public compiler behavior unchanged. |
| Escaping values and language state | Owner tests, matched 28-test generator suites, 32 callback/task tests; local library 1,319 passes/eight existing ignores. |
| Feature compatibility | 178 no-JIT tests with exact membership and source/build identities. |
| Resource effect | Ninety small and sixty extended outcomes; 38,400 extended observations. At 1,024 JIT VMs baseline requested-live/executable growth is 4,194,304 bytes, candidate zero after every drop; VM-only/public-compiler controls unchanged. |
| Test-size benchmarks | 150/150 on candidate, then 150/150 again after canonical recursive-Brainfuck correction. |
| Selected production | Eleven JSON/QuickSort/MergeSort outcomes on frozen candidate; original port identity retained, later Brainfuck-only correction separately covered at test size. |
| Linux release | Run `41611840423`, all seven jobs pass; 1,358 optimized tests/eight existing ignores; formatting, Clippy, MSRV, parity and minimal/full release gates. |
| Platform compatibility | Five verified targeted outcomes above. |
| Integration fidelity | Empty full runtime/test/Cargo diff against final tested platform source. |

Detailed evidence remains in `VM_RESOURCE_DIAGNOSTICS_2026_11_03.md`,
`VM_MEMORY_LINUX_GATE_2026_11_03.md`, `VM_MEMORY_PRODUCTION_2026_11_03.md` and
`VM_MEMORY_PLATFORM_PARTIAL_2026_11_03.md`, with immutable archives. The new
`vm-memory-platform-final-2026-11-03.json.gz` retains final platform logs/API
provenance, verification receipts and source equality evidence.

## Limits and next work

- Pinned Cranelift's Windows allocation owner lacks corresponding deallocation.
  Windows lifecycle success is not reclamation proof; that backend gap remains.
- Public standalone JitCompiler destruction retains its existing contract.
- Requested-byte/mapping reclamation is not lower RSS, faster execution or a
  universal absence-of-leaks claim. Higher RSS samples remain in the evidence.
- Full final-source production, clean-host competition, broader resource and
  architecture gates remain open. No release, tag or upstream contact.
- Shared target executables can be stale. Frozen tested memory CLI SHA is
  `17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`; use recorded
  binary identities rather than assuming a target executable is current.

Next runtime work is the exclusive code-owner reuse candidate described in
`JIT_OWNER_REUSE_BOUNDARY_2026_11_03.md`, with separate regression/resource gates.
The isolated Brainfuck array jump correction can now proceed as a separate port
change with its own checks. No new optimization campaign is silently declared done.
