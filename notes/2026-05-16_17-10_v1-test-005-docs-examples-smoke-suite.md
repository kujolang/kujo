# Kujo Field Notes — V1-TEST-005 docs/examples smoke suite

**Date:** 2026-05-16
**Session:** 17:10 local
**Branch/Commit:** main / cc2a21d
**Scope:** Implemented automated smoke coverage for repository examples and fenced Kujo docs snippets, with explicit run/parse/expected-fail metadata to lock documentation drift intentionally.

---

## What I Changed
- Added `tests/docs_examples.rs`.
- Implemented recursive example inventory over `examples/**/*.kujo`.
- Added explicit example classification surfaces:
  - `Run` set (small curated non-interactive scripts)
  - `ParseOnly` default
  - `ExpectedFail` list for currently known legacy examples with syntax drift
- Added markdown snippet extraction for `README.md` and every `docs/*.md` file by fenced ` ```kujo ` block index.
- Added explicit expected-fail metadata for currently non-parsing docs blocks.
- Added stable smoke execution behavior:
  - `Run` => `kujo run --interpreter`
  - `ParseOnly` => `kujo check --quiet`
  - `ExpectedFail` => assert parse-check fails
- Updated `README.md` testing section with `cargo test --test docs_examples` command.
- Updated `ROADMAP.md` and `CHANGELOG.md` with `V1-TEST-005` completion details.

## Gotchas (Read This Next Time)
- **Gotcha:** Expected-fail metadata can become stale in both directions.
  - **Symptom:** Smoke test fails with “expected-fail now passes” or “unexpected parse failure.”
  - **Root cause:** Docs/examples evolve while expected-fail block/index lists remain static.
  - **Fix:** Re-run inventory sweeps and reclassify affected items in metadata lists.
  - **Prevention:** Treat `ExpectedFail` lists as temporary debt registers, not permanent exemptions.

- **Gotcha:** Docs snippet IDs are ordinal and can shift when earlier blocks are edited.
  - **Symptom:** An unrelated docs edit causes expected-fail IDs to mismatch.
  - **Root cause:** Block IDs use `<file>#<ordinal>` ordering in extraction.
  - **Fix:** Recompute failing snippet IDs after docs edits and update metadata.
  - **Prevention:** Keep snippet extraction deterministic and rerun `cargo test --test docs_examples` after docs updates.

## Things I Learned
- Parse-checking all example files is cheap enough to run in regular CI/local loops, while full run-execution should stay curated because many examples are interactive or environment-coupled.
- Documentation snippet extraction catches drift that plain example-file sweeps miss.
- Explicitly asserting when expected-fail items start passing is useful: it forces deliberate cleanup instead of leaving stale failure metadata.

## Debug Notes (Only if applicable)
- **Failing test / error:** Initial docs expected-fail list had stale block indices for `docs/MEMORY.md`.
- **Repro steps:** Run `cargo test --test docs_examples`.
- **Breakpoints / logs used:** Generated block-failure inventory via shell extraction/check sweep.
- **Final diagnosis:** Snippet ordinal IDs had shifted; refreshed expected-fail block IDs to the current extraction order.

## Follow-ups / TODO (For Future Agents)
- [ ] Convert currently expected-fail example files to parse-clean syntax and move them to parse/run classifications.
- [ ] Reduce expected-fail docs snippet count by updating stale examples in `docs/MEMORY.md`, `docs/CONCURRENCY.md`, and related pages.

## Links / References
- Files touched:
  - `tests/docs_examples.rs`
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
- Related docs:
  - `notes/README.md`
  - `ROADMAP.md`
