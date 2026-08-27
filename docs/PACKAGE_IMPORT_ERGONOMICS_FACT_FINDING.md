# Kujo/Kennel Package Import Ergonomics Fact Finding

Evidence date: 2026-08-27

## Executive Summary

The previous `KUJO_MODULE_PATH` requirement was caused by Kujo's resolver, not
by Ollama or Anthropic. Kennel already records an exact installed dependency
graph in `kennel.lock`, and installs each resolved package under
`kennel_packages/<lock-entry-name>`. A small Kujo-only change can use that
locked state as project-scoped search roots without changing Kennel's resolver,
package exports, or provider packages.

## Current Kennel Install Layout

Kennel installs direct and transitive dependencies into the project-local
`kennel_packages/` directory. Each lock entry records `name`, `install_path`,
source/ref, repository, and resolved commit. The installed package keeps its
own `kennel.toml`; `[package.exports]` is currently package metadata and does
not create runtime namespaces. Provider compatibility shims therefore remain
necessary for root imports such as `from anthropic import messages`.

## Current Kujo Resolution Algorithm

The module loader checks the active importing module's package root first, then
its configured roots. Before this change those roots were `.` and `./modules`,
followed by `KUJO_MODULE_PATH` and any entry-file roots added by the CLI. A
module name is checked as a flat `<name>.kujo` file, then as a dotted nested
path. Canonicalization and root-containment checks reject traversal and symlink
escapes.

## Reproduction Evidence

A clean project with Anthropic `v0.1.1` installed through Kennel produced:

- `kennel_packages/anthropic/`
- `kennel_packages/ai-sdk/`
- a lockfile with exact commits

Running the installed consumer from the project directory with
`KUJO_MODULE_PATH` unset failed with `Module not found: anthropic`. The same
consumer passed when the two installed roots were supplied through
`KUJO_MODULE_PATH`. This reproduces the prior provider reports.

After the Kujo change, the same installed consumer passed with
`KUJO_MODULE_PATH` explicitly unset.

## Why KUJO_MODULE_PATH Is Currently Required

The resolver had no relationship to Kennel's project-local lockfile or install
directory. It searched only process defaults, explicit environment roots, and
CLI-added entry roots. Kennel did not need to change: its lockfile already
provided the deterministic graph and its install paths were safe to derive.

## Root Shim Analysis

Root shims remain necessary under current Kujo semantics. `[package.exports]`
does not currently rewrite `from <package> import ...` into a source path, and
the runtime only loads files and their explicit exports. Removing shims would
be a separate namespace/export feature, not part of automatic root discovery.

## Package Export Analysis

Kennel exports are metadata/validation information today, not runtime-aware
namespace maps. Automatic discovery deliberately does not interpret exports;
it discovers only the locked package directory, preserving existing shims and
avoiding a second runtime export system.

## Transitive Dependency Analysis

Every resolved lock entry contributes its own installed package root. Thus an
Anthropic root import and its `src.ai_sdk` transitive import resolve without
providers concatenating paths. Unlocked directories are ignored.

## Namespace / Alias / Version Analysis

Runtime namespace is the lock entry/install name, which is also Kennel's alias
for the installed directory. Kennel prevents duplicate aliases in normal
workflows. The resolver sorts locked entries by name, de-duplicates repeated
names deterministically, and does not infer namespaces from repository URLs or
manifest exports. Multiple versions under one alias are not representable in
the current flat install layout and remain a Kennel conflict concern.

## Security Analysis

The implementation is project-scoped: it finds only the nearest ancestor with
`kennel.toml` or `kennel.lock`, reads no network state, and does not walk
unrelated parent projects. It accepts only one-component relative install
paths of the form `kennel_packages/<name>`, requires an existing directory,
canonicalizes it, and requires containment within the project's
`kennel_packages` root. Stale or unlocked package directories cannot silently
become search roots. Existing module-name traversal and symlink checks remain
in force.

## Backward Compatibility Analysis

`KUJO_MODULE_PATH` remains unchanged and keeps its existing precedence before
lockfile-discovered roots. Local `.` and `./modules` roots remain first, and
the active importing package root remains highest for nested imports. Projects
without a Kennel lockfile behave exactly as before.

## Architecture Options Considered

| Option | Decision | Reason |
|---|---|---|
| Kujo scans `kennel_packages/` | Rejected | stale/unlocked contents could hijack imports. |
| Kennel writes runtime configuration | Rejected | adds generated state and cross-tool coupling. |
| `kennel run` injects paths | Rejected | ordinary `kujo run` would still be surprising. |
| Kujo reads the nearest lockfile | Chosen | deterministic, offline, project-scoped, and uses existing truth. |
| Keep explicit `KUJO_MODULE_PATH` only | Rejected | preserves unnecessary provider setup. |

## Recommended Architecture

Kujo should add one derived search-root phase after local and explicit roots:
read the nearest lockfile, validate each recorded install path, and add only
existing canonical package roots. Kennel remains responsible for resolution
and installation; Kujo remains responsible for runtime imports.

## File-Level Change Plan

- `kujo/src/module.rs`: discover validated lockfile package roots and add them
  after explicit roots; retain path security and add unit coverage.
- Kujo language and architecture docs: define project boundary and precedence.
- Ollama/Anthropic installed-package scripts: remove test-only manual path
  injection and prove normal consumer behavior.
- Kennel source: no change required.

## Test Plan

Run module unit tests, Kujo import/runtime integration tests, Kennel core and
contract gates, and both provider clean-room scripts. Include locked-only,
missing, traversal, symlink, local precedence, transitive, reinstall, and
`KUJO_MODULE_PATH` compatibility cases.

## Migration Plan

No migration is required. Existing projects may keep `KUJO_MODULE_PATH`; after
upgrading Kujo, normal Kennel projects can remove manually assembled package
roots. Existing root shims remain valid.

## Risks

The resolver currently uses a flat install namespace, so aliases and duplicate
versions retain Kennel's existing constraints. A malformed lockfile is ignored
for automatic roots and produces the normal import diagnostic. A future
runtime-aware exports feature would need an explicit compatibility design.

## Implementation Recommendation

Implement the lockfile-aware Kujo resolver change. It is generic, small,
offline, backward-compatible, and avoids provider-specific or Kennel-specific
runtime branches.

## Should We Implement This Now?

YES
