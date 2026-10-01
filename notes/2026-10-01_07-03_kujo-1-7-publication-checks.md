# Kujo Field Notes — 1.7 publication checks

**Date:** 2026-10-01
**Session:** 07:03 local
**Branch/Commit:** codex/release-v1.7.0-evidence / signed source 813072040a1ac643312f5163fcfa4f26474c9095
**Scope:** Authorized native/npm release and independent registry exception

---

## What I Changed

Recorded the owner's explicit `UNBLOCK_V1_RELEASE`, the verified signed tag,
and the crates.io dry-run exception. Stable defaults remain 1.6.0 while native/npm
publication and installed-artifact verification are pending. No release tag is
rewritten and no crates.io package was published.

## Gotchas (Read This Next Time)

Tag creation is not publication. The signed merge tree exactly matches the
candidate, but tagged artifacts must carry the signed merge commit as provenance.
Cargo normalizes away local registry patches; a repository build alone does not
prove that the registry package compiles.

## Things I Learned

The versioned candidate passed all five installed adapter tests, with three real
canonical tools and exact receipt lookups per target. The tagged full gate passed
again. Registry packaging still loses the tiny_http read-timeout extension.

## Debug Notes (Only if applicable)

- Local `git tag -v v1.7.0` with the registered SSH signer: passed.
- GitHub tag-object verification: verified=true, reason=valid.
- Tagged gate job 110331068275: passed; both fixtures passed 150 with 11 skipped.
- Optional cargo-audit/cargo-deny: unavailable and skipped, not audited.
- `cargo publish --dry-run --locked` from exact signed source: exit 101;
  upstream tiny_http 0.12.0 lacks `Server::http_with_read_timeout` in both VM and
  interpreter. Native/npm builds use the checked-in patch. Release policy section
  8 permits the separate native release with this registry exception recorded.
- An initial dry run used the tree-identical candidate; the failure was reproduced
  from the exact signed merge commit before recording the registry result.

## Follow-ups / TODO (For Future Agents)

Finish tagged native/npm publication and published-install verification before
updating stable defaults. The existing Cargo registry blocker must be solved
without removing HTTP read deadlines. Public ChatGPT local installation remains
a separate host requirement, not a consequence of npm package acceptance.

## Links / References

- [Tagged publication workflow](https://github.com/kujolang/kujo/actions/runs/36850604536)
- [Candidate installation matrix](https://github.com/kujolang/kujo-openai/actions/runs/36822256533)
- [Release notes and registry exception](../docs/RELEASE_NOTES_1_7.md)
- [Release process](../docs/RELEASE_PROCESS.md)
