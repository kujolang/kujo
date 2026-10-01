# Kujo Field Notes — 1.7 candidate preparation

**Date:** 2026-10-01
**Session:** 00:53 local
**Branch/Commit:** codex/release-v1.7.0 / ebd9d4f81a21962f58bdf2058e2a98d36d24ddd9
**Scope:** Unpublished runtime version preparation for portable Ability hosts

---

## What I Changed

Prepared Cargo/lock/npm identity 1.7.0, candidate metadata, release notes and
changelog in an isolated worktree based on
`0c67dd4b840d93431707f732dc25c0c4ea4a8078`. README/roadmap source identity
changes; published stable references and installer default stay 1.6.0.
Historical 1.6.0 metadata remains intact; the identity contract selects the
current version's metadata.

## Gotchas (Read This Next Time)

The release-state guard checks development and published-stable versions
separately. Do not change installer defaults before publication. Candidate npm
artifacts previously labeled 1.6.0 are rehearsal-only and must never overwrite
published packages. New field notes require the timestamped filename and ordered
sections enforced by `.github/scripts/check-new-field-notes.sh`.

## Things I Learned

The pre-version cohort passed five-platform optimized native builds and isolated
adapter installation with three canonical tools and exact receipts per platform.
That proves the implementation cohort; it does not prove versioned artifacts or
public ChatGPT installation support.

## Debug Notes (Only if applicable)

Preparation checks:

- `bash .github/scripts/check-release-state.sh`: passed.
- `npm test --prefix npm`: 12 passed, 0 failed.
- `npm run pack:dry-run --prefix npm`: all six packages packed, lifecycle-free.
- `cargo fmt --check`: passed after formatting the generalized identity contract.
- Initial PR field-notes guard rejected the preparation note filename; corrected
  to the required filename/template without changing runtime behavior.

## Follow-ups / TODO (For Future Agents)

Record full versioned release gates and artifact validation after execution.
No publication directive has been supplied; no release/tag/sign-off is performed.
Retain the explicit `UNBLOCK_V1_RELEASE` requirement before publication.

## Links / References

- [1.7 candidate notes](../docs/RELEASE_NOTES_1_7.md)
- [Release policy](../docs/RELEASE_PROCESS.md)
- [Pre-version runtime rehearsal](https://github.com/kujolang/kujo/actions/runs/36811793883)
- [Five-platform installed candidate acceptance](https://github.com/kujolang/kujo-openai/actions/runs/36816195097)
- [Candidate PR](https://github.com/kujolang/kujo/pull/17)
