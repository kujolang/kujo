# Kennel Namespace Plan

The reserved-name system is the foundation for future Kennel package naming and namespace safety.

## What this enables now

- Deterministic blocking of core and first-party names.
- Separation between namespace routing, command ownership, and package identity.
- Explicit first-party trust boundaries.

## Package-name uniqueness

Current `kujo.toml` parsing rejects reserved package names for third-party manifests.

Future Kennel registries should extend this into global uniqueness checks for:

- unscoped package names
- scoped package names
- transferred/deprecated aliases

## User/org scopes

Planned shape:

- `@user/package`
- `@org/package`

Scoped names should still reject reserved roots and blocked generic identifiers.

## First-party package names

First-party names remain explicitly reserved (for example `kennel`, `spec`, `eval`) and cannot be claimed by third-party publishers.

## Blocked generic names

Generic names (`dev`, `admin`, `tools`, `system`, etc.) are blocked in top-level alias routing to avoid ambiguous command surfaces.

## Future registry validation

When Kennel registry APIs exist, validation should include:

- reserved-name enforcement server-side
- namespace/package collision checks
- scope ownership checks
- signed/trusted first-party package verification

## Migration path from local packs to Kennel packages

1. Keep local workflow packs namespaced and non-reserved.
2. Use `kujo pack run <namespace> <command>` as canonical execution.
3. Publish under scoped Kennel names when available.
4. Keep contributions (for example doctor profiles) in explicit manifest extension points rather than top-level command claims.

