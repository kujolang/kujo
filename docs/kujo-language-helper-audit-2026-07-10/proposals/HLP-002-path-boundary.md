# HLP-002 — symlink-aware path boundary

## Problem and evidence

CaseFile, MCP, Dispatch, SSG, RAG, and Workcell each implement a root check.
CaseFile normalizes components and rejects `..`; MCP uses an absolute-prefix
check; other tools use lexical or canonicalized variants. A lexical check can
be bypassed by symlinks and can mishandle missing targets, Windows drives, or
case rules. Kujo core already has Rust path security for archive extraction, but
that contract is not exposed as a general script primitive.

## Root cause and ownership

This is runtime security semantics. It cannot be safely implemented as a Kujo
string helper. Prototype in Rust and expose only after a platform contract is
written. Classify as filesystem-read for inspection and filesystem-write or
filesystem-delete when used as a guard for effects.

## Proposed API

Proposed syntax:

```kujo
check := path_within(root, candidate, {
    allow_missing_leaf: true,
    follow_symlinks: false
})
if check.ok == false { return Result::Err(check.error) }
```

Recommended initial return: `Result<dict, PathBoundaryError>` where success
contains `root`, `candidate`, `canonical_root`, `canonical_candidate`, and
`existing` metadata only if those fields are available. Failure includes
`code` values such as `outside_root`, `symlink_escape`, `missing_parent`,
`invalid_path`, or `permission_denied`. Prefer a separate guard operation later
if checking and writing can race; a check alone is not a capability guarantee.

## Semantics and security

Define whether the root itself is allowed, whether a missing leaf is allowed,
whether symlinks are rejected or resolved, and how a missing parent is handled.
Use canonical paths for existing components and a safe lexical normalization for
the missing suffix. Never rely only on `starts_with`. The operation must not
mutate filesystem state. A write API should eventually combine the boundary
check with the open/rename operation to avoid time-of-check/time-of-use races.

## Alternatives considered

- Documenting `path_absolute` plus prefix tests: insufficient for symlinks.
- Copying CaseFile’s component normalization: misses platform and canonical
  semantics.
- Reusing archive extraction internals directly: wrong public shape and too
  narrow to archives.
- A broad `safe_write(path, root, …)`: hides security policy; defer until the
  primitive contract is proven.

## Migration

Use CaseFile and MCP as references, then migrate Dispatch source lookup and SSG
output checks. RAG query API and Workcell should follow once missing-target and
symlink behavior is agreed. Migration complexity is medium to high because
some tools intentionally reject symlinks while others permit them.

## Tests and performance

Test equal root, child, sibling-prefix false positive, `..`, absolute/relative
mix, symlinked directory, symlinked file, broken symlink, missing leaf, missing
parent, permissions, Unicode, macOS/Linux, and Windows drive/UNC cases. Add a
race-oriented integration test for a combined guarded write before claiming
security enforcement. Benchmark canonicalization on deep paths; do not optimize
away correctness.

## Recommendation

Prototype and design further. Confidence is medium because the evidence is
strong but the cross-platform contract is not yet stable.
