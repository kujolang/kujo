# HLP-001 — atomic bounded file write

## Problem and evidence

Seven mature or usable tools implement replacement writes locally. PackWrite,
Redact, Workcell, and RAG scripts delete before calling `write_file`; Tribunal
uses a temp path then rename; CaseFile and MCP wrap writes with result/error
objects. Core `write_file` supports an explicit overwrite flag but performs a
direct write. These behaviors differ on interruption, permissions, parent
directories, and cleanup.

## Root cause and ownership

This is a runtime filesystem semantic, not a language syntax problem. Own it in
the core runtime/standard library because correct replacement requires OS-level
rename and cleanup behavior that Kujo code should not reimplement.

## Proposed API

Proposed syntax, not current Kujo syntax:

```kujo
result := write_file_atomic(path, content, {
    overwrite: true,
    create_parents: true,
    max_bytes: 10485760
})
```

Recommended signature: `write_file_atomic(path: string, payload: string|bytes,
options?: dict) -> Result<bool, FileError>`. A bytes-specific sibling may be
needed if union payloads are not stable. Return `Ok(true)` after the rename;
return `Err` with `code`, `path`, `stage`, and `message` fields. Do not silently
delete the destination. Default `overwrite` should be false to match current
`write_file` safety. The operation is effectful, capability-gated as
filesystem-write, non-mutating on failure where the OS permits, and deterministic
with respect to payload/path/options.

Implementation should create a uniquely named sibling temp file, write and
flush it, apply the intended permissions policy, rename it over the destination
only when allowed, and remove the temp file on failure. Document that rename
atomicity is filesystem-dependent and that directory durability is not promised
unless an explicit future option exists.

## Edge cases and security

Reject empty paths, directories, oversized payloads, invalid option types, and
existing destinations when overwrite is false. Do not follow a destination
symlink without a documented policy. Do not expose temp names in error messages
unless useful for debugging. Test permissions, disk-full/partial-write paths,
concurrent writers, symlinks, parent creation, Unicode names, and Windows path
behavior.

## Alternatives considered

- Delete then `write_file`: unsafe on interruption and loses the old file.
- Temp + rename in Kujo: repeats OS-sensitive code and cannot guarantee cleanup.
- A broad `safe_write`: hides atomicity, root policy, overwrite, and size
  semantics; rejected.
- Package-only implementation: cannot provide consistent runtime atomicity or
  parity; useful only as an interim shim.

## Migration

Migrate Tribunal’s `write_text` first because it already expresses temp + rename,
then CaseFile and MCP guarded writes. Keep local wrappers when they add root or
redaction policy, but make them call the atomic primitive. Do not migrate simple
new-file writes automatically; overwrite behavior must be reviewed. Estimated
complexity is low to medium and most call sites can remain unchanged behind a
wrapper.

## Tests and performance

Add unit and integration tests for normal text/bytes, empty payload, overwrite
false/true, missing parent, parent-as-file, permissions, cleanup after write
failure, symlink destination, concurrent writers, size limits, and VM/interpreter
parity. Benchmark small and 10 MB payloads against direct `write_file`; the
extra temp write is expected and acceptable for correctness-sensitive artifacts.

## Recommendation

Implement after a focused runtime prototype. Confidence is high; the API should
remain narrow and should not absorb path confinement or redaction policy.
