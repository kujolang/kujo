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

## Implemented API

`write_file_atomic(path, content_or_bytes, overwrite?) -> bool` is registered
in the core runtime. The optional overwrite flag defaults to false. Text and
bytes payloads share the same bounded implementation; successful writes return
`true`, while validation and filesystem failures return the existing
`Value::Error` runtime shape. The operation is capability-gated as
filesystem-write.

The implementation creates a uniquely named sibling temp file, writes and
flushes/syncs it, then renames it over the destination when overwrite is true.
The no-overwrite path hard-links the completed inode into place so a destination
appearing during finalization is not clobbered. Temporary files are removed on
failure. Directory durability is not promised beyond the file sync.

## Edge cases and security

Reject empty paths, directories, oversized payloads, invalid payload types, and
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

## Status

Implemented and pushed in the Kujo core. The API remains narrow and does not
absorb path confinement or redaction policy.
