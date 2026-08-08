# Kujo Deprecation Policy

Status: stable v1.0.0 policy
Last updated: 2026-05-01

This policy defines how Kujo deprecates CLI, LSP, and runtime/native-function surfaces.

## Scope

This policy applies to:

- CLI commands, flags, and machine-readable JSON fields.
- LSP method behavior and protocol payload fields.
- Runtime language and native standard-library surfaces.

## Warning Format

Deprecations must use explicit, stable warning text that includes all of the following:

- the deprecated surface identifier (command/flag/field/function/feature)
- the replacement surface when available
- the first Kujo version that marks the surface deprecated
- the earliest Kujo version where removal is allowed

Canonical human-readable warning shape:

`DEPRECATION: <surface> is deprecated since <version>; use <replacement>; removal no earlier than <version>.`

For machine-readable CLI JSON outputs, deprecation warnings should be emitted on `stderr` (not mixed into successful JSON payloads on `stdout`).

## Versioning Windows (Semver)

Kujo follows semantic versioning. Deprecation timing rules:

- `0.x` phase: deprecation windows may be shorter, but must still include a warning and changelog note before removal.
- `1.x` and later: deprecated surfaces must remain available for at least one full minor release after the deprecation release.

Examples:

- If deprecation starts in `1.2.0`, earliest removal is `1.3.0`.
- If deprecation starts in `1.2.3`, earliest removal is `1.3.0`.
- Removal in `2.0.0` is always allowed for previously deprecated `1.x` surfaces with migration notice.

Breaking removals or incompatible behavior changes must only happen in a major release unless a previously documented temporary exception applies.

## Required Change Set For Any Deprecation

A deprecation is not complete unless all items below land together:

- update relevant docs (`README.md` and affected contract/reference docs)
- add/update regression tests for warning and fallback behavior where applicable
- add `CHANGELOG.md` entry with migration guidance
- include release note text with the exact removal floor version

## Removal Rules

Before removing a deprecated surface:

- verify the minimum deprecation window has elapsed
- verify replacement path is documented and tested
- remove or update stale docs and examples in the same change
- include a changelog note that points users to the replacement

## Exceptions

Immediate removal without deprecation window is allowed only for:

- security-critical break-glass removals
- data-loss or integrity bugs where keeping behavior is unsafe

These exceptions still require same-release documentation and changelog callouts explaining why the normal window was skipped.
