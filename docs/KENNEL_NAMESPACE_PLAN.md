# Kennel Namespace Plan

The reserved-name system is the foundation for future Kennel package naming and namespace safety.

## v1.0 Boundary

Kennel is not a public package registry in Kujo v1.0. The v1.0 launch boundary
is:

- local `kujo.toml` manifest parsing
- deterministic `kujo.lock` generation and `--frozen` verification
- reserved-name enforcement for packages, namespaces, commands, and first-party
  workflow pack identities
- local workflow-pack discovery and execution

Public registry APIs, namespace ownership accounts, registry authentication, remote package resolution, upload transport, and signed package distribution are future Kennel work, not v1.0 release promises.

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

When Kennel registry APIs exist in a future release, validation should include:

- reserved-name enforcement server-side
- namespace/package collision checks
- scope ownership checks
- signed/trusted first-party package verification

## Migration path from local packs to Kennel packages

1. Keep local workflow packs namespaced and non-reserved.
2. Use `kujo pack run <namespace> <command>` as canonical execution.
3. Publish under scoped Kennel names when a future registry exists.
4. Keep contributions (for example doctor profiles) in explicit manifest extension points rather than top-level command claims.
