# Kujo Field Notes — V1-MOD-001 module import hardening

**Date:** 2026-05-13
**Session:** 01:13 local
**Branch/Commit:** main / 1682837
**Scope:** Hardened module import path resolution and cache behavior in the runtime module loader. Added regression coverage for package-root-relative imports, traversal rejection, symlink escape rejection, circular import chain diagnostics, and cache refresh after source updates.

---

## What I Changed
- Updated module loading in `src/module.rs` to use deterministic search ordering:
  - importing module package root first
  - then configured module search paths
- Reworked module cache key semantics to include package-root context plus canonical module path.
- Added source-metadata-based cache invalidation (`mtime` + file length) before cache reuse.
- Improved circular import detection to emit explicit chain diagnostics (`a -> b -> a`).
- Added/updated module regression tests in `src/module.rs` for:
  - missing symbol errors
  - traversal module-name rejection
  - symlink escape rejection
  - package-root-relative import preference
  - cache refresh after source changes
- Added integration regressions in `tests/package_module_workflow_integration.rs` for:
  - cycle-chain diagnostics via `kujo run --interpreter`
  - module refresh after file changes across repeated runs

## Gotchas (Read This Next Time)
- **Gotcha:** Cache keying by module name alone causes incorrect module reuse across package contexts.
  - **Symptom:** Importing the same module name from two package roots could return exports from the wrong package.
  - **Root cause:** Cache identity did not include package/search-root context.
  - **Fix:** Use a cache key composed of package root + canonical module path.
  - **Prevention:** Any module-cache refactor must preserve package-root scoping and add dual-package same-name regressions.

- **Gotcha:** Circular-import diagnostics are hard to debug if only the current module name is reported.
  - **Symptom:** Errors like `Circular import detected: x` did not show where the cycle came from.
  - **Root cause:** Cycle detection did not materialize the active import stack chain.
  - **Fix:** Build a chain from the first repeated module through the current import (`a -> b -> a`).
  - **Prevention:** Keep stack metadata rich enough to produce diagnostic chains, not single-node messages.

## Things I Learned
- Module search order is a language/runtime contract surface, not just an implementation detail.
- Canonical path containment checks are necessary but not sufficient by themselves; package context must also scope cache identity.
- Metadata-based cache invalidation is a practical, deterministic baseline for module reload behavior in test and CLI workflows.

## Debug Notes (Only if applicable)
- **Failing test / error:** Rust compile failed in `src/module.rs` test section with parser errors (`prefix ... is unknown`, unterminated string context) after file corruption in a previous edit state.
- **Repro steps:** `cargo test --lib module::tests::`.
- **Breakpoints / logs used:** Read file slices around the failing line range and replaced corrupted module section with a coherent implementation.
- **Final diagnosis:** File tail had a broken/truncated test block that removed/overlapped valid code; rewrite fixed structure and restored compilation.

## Follow-ups / TODO (For Future Agents)
- [ ] If module-alias syntax is introduced later, explicitly define whether aliasing affects cache keys.
- [ ] Consider stronger cache invalidation (content hash) if metadata granularity causes stale-cache edge cases on specific filesystems.

## Links / References
- Files touched:
  - `src/module.rs`
  - `tests/package_module_workflow_integration.rs`
  - `docs/LANGUAGE_SPEC.md`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
