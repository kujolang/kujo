# Kennel Namespace Policy

The public Kennel registry is live at
[kennel.kujolang.ai](https://kennel.kujolang.ai/). This document separates the
historical Kujo 1.0 boundary from the namespace work that Kennel still owns.

## v1.0 Boundary

Kennel was not a public package registry when Kujo v1.0 shipped. The v1.0
runtime boundary was:

- local `kujo.toml` manifest parsing
- deterministic `kujo.lock` generation and `--frozen` verification
- reserved-name enforcement for packages, namespaces, commands, and first-party
  workflow pack identities
- local workflow-pack discovery and execution

That historical boundary does not describe the current ecosystem. Kennel now
provides public registry reads, exact release resolution, checksums, local
project installs, and global tools. Kujo's built-in `package-publish` command is
still a metadata preview and does not upload to Kennel.

## Current boundary

- Registry and publishing policy belongs in Kennel, not the Kujo runtime.
- Kujo owns the bounded filesystem, locking, publication, symlink, process, and
  import mechanisms that Kennel uses.
- Public reads do not imply open third-party publishing.
- Third-party accounts, organization scopes, private packages, transfers, and
  signed distribution are not current release promises.

## Package-name uniqueness

Current `kujo.toml` parsing rejects reserved package names for third-party manifests.

Future Kennel publishing work should extend this into global uniqueness checks
for:

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

## Future publishing validation

When third-party publishing is opened, validation should include:

- reserved-name enforcement server-side
- namespace/package collision checks
- scope ownership checks
- signed/trusted first-party package verification

## Migration path from local packs to Kennel packages

1. Keep local workflow packs namespaced and non-reserved.
2. Use `kujo pack run <namespace> <command>` for local workflow packs.
3. Use Kennel for packages and global tools that have registry releases.
4. Add scoped publishing only after account and ownership policy ships.
5. Keep contributions (for example doctor profiles) in explicit manifest
   extension points rather than top-level command claims.
