# Isolated VM code-memory candidate — not accepted

This branch contains a proposed VM-private code-owner reclamation change. It is
not merged into the accepted performance branch and is not a release. Production
and platform acceptance remain pending. Public JitCompiler destruction retains
its existing behavior, including the documented backend Windows limitation.

The VM owns its compiler and native pointer caches exclusively. Native calls
borrow the VM synchronously; dropping the VM cannot overlap an active call.
Returned values, generators and saved execution state preserve bytecode and
language values, not this compiler's native entry pointers. On destruction, the
VM takes its compiler once and releases generated code/data. Remaining pointer
cache destruction does not invoke those entries. The compiler's embedded literal,
field/path/shape metadata survives through the free operation and is then dropped.
No externally callable raw entry escapes through the VM's public execution API.

## Explicit unsafe-site review

Six executable inventory matches are added; the exact total changes from269 to275:

- `src/jit.rs`: unsafe method declaration and backend `free_memory` call (147→149).
  The method is crate-private and consumes the compiler; its caller must prove
  no generated entry is executing or can be invoked again. It is used only by
  the owning VM's destruction path, not public compiler destruction.
- `src/vm.rs`: one destruction call and three macOS test sites (15→19).
  The call consumes the uniquely owned compiler after synchronous execution.
  The test ABI declaration matches Mach region querying. Its query uses valid,
  initialized output buffers of sufficient size and never reads code memory.
  Its port-deallocation call releases only the send right returned by that query,
  and only when nonzero. A subprocess prevents parallel tests from reusing freed
  addresses during mapping assertions. Reclamation is checked by execute
  permission, because an allocator can retain non-executable pages.

The exact total and per-module assertions remain enforced; unrelated module
budgets are unchanged. The inventory and TODO tables are regenerated to update
line positions and these reviewed entries, not to hide new findings.

## Existing local evidence

Accepted performance branch commit80a05a3c records the full evidence in
`benchmarks/langarena/VM_RESOURCE_DIAGNOSTICS_2026_11_03.md` and its archives:
1,319 library passes,150 test-size checks, matched28-test generator suites,
32 callback/task checks,178 no-JIT tests,90 small resource outcomes and60 extended
resource outcomes through1,024 owners. These are revision-specific local results,
not platform acceptance. The runtime files match that tested candidate patch;
this preparation adds inventory review/tests and documentation only.

The live-byte and executable-map probes return to baseline after every candidate
VM destruction while the baseline retains4,096 bytes per JIT VM. Public compiler
retention stays unchanged. These counts do not establish lower RSS or speed.

Next: run exact-branch platform checks and production controls, preserving all
failures/timeouts. Do not integrate or publish before remaining acceptance gates.

## Platform verification branch

The platform follow-up extends the executable-entry regression to Linux by
reading `/proc/self/maps` metadata in the isolated test subprocess. It checks
execute permission before and after each VM destruction, without dereferencing
retired code or adding unsafe sites. Runtime implementation outside the test
module remains byte-for-byte identical to candidate75445306. Windows runs the
retained-value/code-toggle and integration contracts, but has no reclamation
claim: the backend limitation remains explicit.

The manual `native-upgrade` workflow accepts an optional `code_lifetime_only`
Boolean (default false). True calls the separate lifetime workflow on Linux
x64/arm64, macOS x64/arm64 and Windows x64. Existing push/PR/default manual upgrade
behavior remains unchanged. This route verifies owner, generator, callback and
task contracts and uploads logs; it neither publishes artifacts nor releases.
Platform results remain pending until exact-commit jobs and evidence are reviewed.

Platform run `41613864999` reached all five workers. Windows failed before
compilation: checkout converted Rust files to CRLF, violating the repository's
rustfmt newline policy. The raw job log is retained under local evidence as
`windows-lifetime-checkout-failure.log`. Configure `core.autocrlf=false` before
checkout; preserve the formatting gate. A Windows-only manual retry option avoids
restarting the four still-running Linux/macOS jobs. Runtime and test bodies are
unchanged by this workflow repair; Windows semantic verification remains pending.

Windows-only retry `41614100004` passed checkout but still failed formatting for
vendored `tiny_http`'s `task_pool.rs`: its nested `.gitattributes` specifies
`* text=auto`, so disabling autocrlf alone leaves native Windows line endings.
Set `core.eol=lf` before checkout as well. The tracked blob has LF endings; no
vendored source or formatting requirement is changed. Preserve the retry log.

Run `41614521350` passes formatting but fails while building vendored OpenSSL:
Git Bash selects its incomplete MSYS Perl installation (missing
`Locale/Maketext/Simple.pm`). Use native PowerShell for Windows cargo steps,
matching existing Windows CI, and a folded single-line integration command.
All default features and all ten suites remain required. This is build-environment
repair; no tests ran and no runtime failure/pass is claimed. Artifact `13220463449`
preserves the exact source/toolchain and build failure.

The first native-shell edit was rejected at dispatch (HTTP 422): the `runner`
context is unavailable in a `shell` expression. No run started. Remove that
expression and use GitHub's native platform default shell for both Cargo steps
(PowerShell on Windows, Bash on Unix); keep the folded single command.
