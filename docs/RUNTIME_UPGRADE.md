# Native runtime upgrades

`kujo upgrade` installs the latest published stable release from
`kujolang/kujo`. It updates only the running standalone runtime executable.
It never builds from source, changes ecosystem source snapshots, `install.json`
source refs, profiles, package pins, or project dependencies. Invoking the
command authorizes installation; there is no interactive prompt.

```sh
kujo upgrade
kujo upgrade 1.2.4
kujo upgrade v1.2.4
kujo upgrade --check --json
kujo upgrade 1.2.4 --check
kujo upgrade 1.2.3 --allow-downgrade
```

Examples name versions, not promises that a future release is published.
Only exact stable `MAJOR.MINOR.PATCH` versions, optionally prefixed by `v`, are
accepted. Ranges, prereleases, and build metadata are rejected. Latest resolves
through GitHub's latest-release API; exact versions use the release-by-tag API.
Draft, unpublished, and prerelease metadata are rejected. Missing artifacts or
checksums cause failure without a source-build fallback.

The five release targets are Linux x64/arm64, macOS x64/arm64, and Windows x64.
Same-version requests are successful no-ops. Latest never downgrades a newer
local runtime, even with `--allow-downgrade`. An explicit older target requires
`--allow-downgrade` to install. `--check` may inspect it without that flag.

## Installation ownership and symlinks

The destination is the canonical path of the executable actually running, not
an assumed installation directory or a PATH search. Invoking through a symlink
updates its resolved target and preserves the symlink. The destination is
reported in both human and JSON output.

npm paths and nearby `@kujolang/kujo-*` package manifests are recognized;
Cargo paths and nearby `.crates.toml`/`.crates2.json` receipts are recognized.
Those installations receive package-manager guidance instead of being changed:

```sh
npm install --global @kujolang/kujo-runtime@VERSION
cargo install kujolang --version VERSION --locked --force
```

For a project-local npm install, use that project's package manager and scope.
Development `target` directories and recognized Homebrew, Nix, Scoop,
Chocolatey, Snap, WinGet, WindowsApps, and system `/usr/bin`/`/bin` locations
are refused. `--check` still reports release availability and installation kind
for these installations, without changing them.

Legacy standalone installations (including `~/.local/bin/kujo`) have no
universal ownership receipt. A running executable named `kujo`/`kujo.exe`, with
no recognized manager marker, is treated as legacy standalone. This is a
compatibility heuristic, **not proof of ownership**: custom package managers
and relocated package binaries without their metadata cannot always be
identified. Use the original package manager for those installations. A custom
Cargo home is recognized through its Cargo receipts. No ownership claim or
runtime receipt is written into ecosystem `install.json`.

## Verification, replacement, and recovery

Requests require HTTPS, allow at most five redirects, use a 10-second connect
and 120-second total request timeout, and bound metadata to 2 MiB, checksums to
4 KiB, archives to 128 MiB, and extracted executables to 256 MiB. Release asset
URLs must name the exact official repository/tag/asset. Archive length must
match metadata and its SHA-256 must match the published per-asset checksum
before extraction or execution. This verifies integrity through the official
release channel; it is not independent artifact signing.

ZIP64, multi-disk ZIPs, and ZIP comments are not accepted; official Windows
artifacts do not need these extensions. TAR expansion including padding is
bounded to 257 MiB. Native Rust TAR/gzip and ZIP readers accept only the single expected regular
executable. Traversal, absolute paths, links, extra entries, duplicates, and
oversized binaries are rejected. Staging takes place beside the destination,
so it shares the destination filesystem and does not depend on executable
system temporary directories. The staged `--version` must match within ten
seconds; stdout is limited to 4 KiB and stderr is discarded.

A persistent `.kujo-upgrade.lock` beside the destination uses an OS advisory
lock; another upgrade fails while it is held. The file remains after exit to
avoid unlink/recreation races, but the OS releases its lock even after a crash.
The destination's file identity, metadata, hash, and ownership classification
are checked again before replacement. These checks coordinate cooperating
upgraders; they do not sandbox a hostile process with directory-write access.

Successful installation retains a uniquely named `kujo-backup-UUID` (Windows:
`.exe`) beside the destination, reported as `backup` in JSON. Unix copies and
syncs the prior executable before replacing the destination with a same-directory
rename. Ordinary replacement failure leaves the old destination intact.
Windows moves the running executable to the retained backup, then renames the
staged executable into place; failure attempts to restore the old name and
reports the recovery path if restoration also fails. No helper process or
self-deleting executable is needed. Keep backups until the new runtime is
confirmed usable; remove them manually when no longer needed.

To recover, exit Kujo processes, then move the reported backup to the reported
destination (preserving executable permissions on Unix). Windows has a brief
interval between the two renames when the destination name is absent; a crash
in that interval requires this manual recovery. No universal crash-atomicity
or power-loss durability is promised. Task-owned staging files are removed on
ordinary exit; a killed process may leave `.kujo-upgrade-*` staging files, which
can be removed once no upgrade is running. Permission failures are reported;
Kujo never elevates privileges automatically.

`self-replace` 1.5.0 was evaluated (Apache-2.0, Rust 1.63; current published
version at implementation). Its Windows implementation relocates the executable
before fallible deletion-helper setup/copy operations and has no rollback at
those intermediate failures. The narrower retained-backup replacement above
avoids that dependency and its helper lifecycle. See the
[upstream source](https://github.com/mitsuhiko/self-replace/tree/1.5.0).
Archive dependencies are unconditional because upgrading must work with
`--no-default-features`; `runtime-archive` still gates language archive APIs.

## Bootstrap and validation

Releases predating this command cannot upgrade themselves with it. First install
a release containing the command using the existing installer or package
manager. The user's local `kujo-upgrade` helper is a separate custom tool and
is not replaced or invoked by this command.

Fixture transport is internal to Rust tests. Production has no alternate
executable-download URL or test endpoint flag. Tests use local HTTP fixtures,
synthetic archives, and disposable native executables. They require `rustc`
and, on Linux, `strip` to remove debug data from the disposable test-runner
copy while preserving the production executable size limit. The native upgrade CI
matrix runs these tests, including a copied running-executable replacement,
on all five release targets. The release-binary jobs also run the focused tests
before packaging. Cross-platform results are established by those jobs; a
single local run does not verify the other operating systems.
