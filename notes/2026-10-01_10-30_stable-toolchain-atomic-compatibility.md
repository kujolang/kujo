# Kujo Field Notes — Stable toolchain atomic compatibility

**Date:** 2026-10-01
**Session:** 10:30 local
**Branch/Commit:** codex/stable-v1.7-installation / 5c7c031
**Scope:** Stable compiler compatibility without an MSRV change

---

## What I Changed

CI run 36868620908 rejects three existing Atomic::fetch_update calls because
new stable Rust deprecates that spelling. Retain the operations and memory
orderings to preserve the declared Rust 1.89 MSRV. Scope deprecated allowances
to the three containing functions instead of suppressing warnings globally or
switching to an API unavailable on the minimum compiler.

The installation identity contract passed locally (one test). Latest-stable
Clippy and MSRV CI must pass before merging. Cargo registry publication remains
deferred; this is compiler compatibility for the existing runtime build.

## Gotchas (Read This Next Time)

Do not replace the atomic API without checking Rust 1.89 compatibility.

## Things I Learned

A newer stable compiler can deprecate an API still required by the MSRV.

## Debug Notes (Only if applicable)

Formatting and diff checks passed; the identity contract passed one test.

## Follow-ups / TODO (For Future Agents)

Verify latest-stable Clippy and MSRV CI before merge.

## Links / References

- https://github.com/kujolang/kujo/actions/runs/36868620908
