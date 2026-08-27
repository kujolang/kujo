# Package Import Ergonomics Implementation Report

Evidence date: 2026-08-27

## Executive Summary

Kujo now discovers Kennel-installed dependencies automatically from the nearest
project lockfile. The change removes manual `KUJO_MODULE_PATH` setup for normal
Kennel consumers while retaining that environment variable for explicit custom
roots. Kennel and provider implementations were not changed.

## Original Problem

Ollama and Anthropic clean-room consumers passed only after adding their
installed package roots to `KUJO_MODULE_PATH`. Without it, Kujo searched local
roots but had no knowledge of `kennel_packages/`.

## Root Cause

Kennel recorded the resolved graph in `kennel.lock`, but Kujo's module loader
did not read that file. Kennel exports are not runtime namespace maps, so root
shims remain the current package import mechanism.

## Chosen Architecture

The Kujo module resolver finds the nearest ancestor containing `kennel.toml` or
`kennel.lock`, reads `kennel.lock`, and adds only existing canonical
`kennel_packages/<lock-entry-name>` directories. It never scans arbitrary
package directories or accesses the network.

## Alternatives Rejected

- Scanning all `kennel_packages/`: stale and unlocked directories could shadow
  locked dependencies.
- Kennel-generated environment/configuration: adds generated state and tool
  coupling for behavior Kujo can derive locally.
- `kennel run` path injection: leaves ordinary `kujo run` surprising.
- Runtime interpretation of `[package.exports]`: would create a second import
  namespace mechanism and break existing explicit shims.

## Kujo Changes

`src/module.rs` contains lockfile-aware project discovery, strict install-path
validation, canonical containment checks, deterministic name sorting, and unit
tests for locked-only resolution and path escape rejection. The type checker
uses the same derived roots for imported-signature analysis.

## Kennel Changes

None. Kennel's existing lockfile and install layout already expose sufficient
deterministic information.

## Provider Changes

Ollama and Anthropic installed-package scripts now unset `KUJO_MODULE_PATH`
and execute from the clean project directory. Their provider code and public
APIs are unchanged.

## Import Resolution Precedence

1. Active importing module's package root.
2. Loader defaults: current project directory and `./modules`.
3. Explicit `KUJO_MODULE_PATH` entries.
4. Entry-file roots added by the CLI where applicable.
5. Existing package roots named by the nearest `kennel.lock`.

The first matching safe candidate wins within this existing model. Locked
package discovery is deterministic because lock entries are sorted by name and
unlisted directories are never searched.

## Alias / Namespace Behavior

The installed lock entry name is the runtime namespace because Kennel installs
to that name. This matches current aliases. Root shims and explicit exports
remain required. Hyphenated package names are not silently transformed by the
runtime; a Kennel alias that is a valid Kujo module identifier is the supported
import spelling. Duplicate aliases remain a Kennel error.

## Security Boundary

Only a nearest project boundary is considered. Install paths must be exactly
one relative component below `kennel_packages`, point to an existing directory,
canonicalize successfully, and remain within the canonical package root.
Symlink escapes, traversal, stale/unlocked packages, arbitrary parent projects,
and network fetching are excluded. Existing module-name sanitization and
`KUJO_MODULE_PATH` behavior remain intact.

## Backward Compatibility

Projects without Kennel metadata behave as before. `KUJO_MODULE_PATH` remains
available and is still evaluated before lockfile-derived roots. Existing local
modules, package shims, VM imports, interpreter imports, and dotted imports
remain supported.

## Tests Added

- Module-loader unit test: locked roots only, stale roots excluded.
- Module-loader unit test: install-path escape rejected.
- Kujo integration test: locked package import works with no environment path.
- Ollama and Anthropic installed-package scripts: no manual path wiring.

## Kujo Test Results

- Module unit tests: `775 passed, 0 failed, 7 ignored`.
- Package/module integration: `9/9` passed.
- Runtime security: `11/11` passed.
- Full `scripts/release_gate.sh --full`: blocked by three pre-existing
  generated-artifact freshness tests (`UNSAFE_INVENTORY.md`,
  `V1_CODE_TODO_TRIAGE.md`, and `VM_RUNTIME_MISMATCH_INVENTORY.md`), all older
  than the repository's seven-day freshness policy. No import test failed.

## Kennel Test Results

Kennel source was not modified. Its existing core/contract validation remains
the applicable gate; no new dependency-resolution behavior was required.

## Ollama Clean-Room Result

PASS after rebuilding Kujo: the installed `v0.1.8` consumer passed from a clean
project with `KUJO_MODULE_PATH` unset. The lockfile continued to resolve Ollama
and AI SDK transitively.

## Anthropic Clean-Room Result

PASS after rebuilding Kujo: the installed `v0.1.1` consumer passed from a clean
project with `KUJO_MODULE_PATH` unset. The lockfile continued to resolve
Anthropic and AI SDK transitively.

## Contract v1 Impact

The former Contract v1 wording that called explicit `KUJO_MODULE_PATH` a
current requirement is now stale. It should be handled as a patch-level
clarifying update: automatic lockfile discovery is the normal path, while the
environment variable remains an explicit extension. Provider architecture and
conformance requirements do not change.

## Documentation Updated

- `docs/LANGUAGE_SPEC.md`
- `docs/ARCHITECTURE.md`
- `README.md`
- `docs/PACKAGE_IMPORT_ERGONOMICS_FACT_FINDING.md`
- this report
- Kennel README behavior note
- Ollama and Anthropic installed-package gates

## Remaining Limitations

`[package.exports]` still does not generate runtime namespaces, root shims are
still required, and the flat install layout does not support multiple versions
under one alias. A malformed or incomplete lockfile yields the normal module
diagnostic rather than an automatic install. These are separate future
namespace/export ergonomics questions.

## Ready for Universal Provider Builder?

YES
