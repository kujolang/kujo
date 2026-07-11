# Agent reliability opportunities

## Canonical safe paths

Generated code should have an obvious `fs.write_file_atomic` and
`fs.path_within` path. Both should require explicit roots/options where a
destructive or out-of-root operation is possible, return structured failures,
and impose size/path limits. This reduces unsafe local “delete then write” and
string-prefix implementations.

## Canonical structured input

Agents frequently receive dictionaries from JSON, AI providers, CLI arguments,
and persisted receipts. HLP-003 should make the expected shape visible in the
call, report a precise data path, and avoid silent coercion. Silent
`normalize_*` helpers are convenient but can hide malformed model output.

## Canonical CLI and process behavior

HLP-004 should parse tokens from an explicit array and return errors rather than
silently consuming a following flag as a value. Process examples should prefer
`spawn_process(['program', 'arg'], options)` over `execute('shell text')`; shell
quoting is intentionally not promoted.

## Deterministic and bounded output

The runtime already provides deterministic JSON, file read/write limits,
process output truncation, AI replay, token estimates, and redacted secrets.
The missing work is discoverability plus a small `read_bounded`/walk API so
agents do not invent unbounded reads or recursive scans.

## Security-specific restraint

Redaction and path policies are easy for agents to call but hard to get right.
HLP-005 keeps policy profiles in a versioned package with audit metadata. Core
`secret` should remain the primitive for values that must never serialize in
plaintext; regex redaction should not be silently applied to all strings.

## Reviewability

Canonical APIs should make effect boundaries visible: filesystem write,
filesystem delete, network request, process spawn, clock read, and secret
reveal. A shorter helper that hides those effects would reduce review quality,
even if it reduces tokens.
