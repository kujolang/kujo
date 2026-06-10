# V1U-OPEN-002 Evidence: ROADMAP Final Checklist Closure

Date: 2026-05-21
Item: `V1U-OPEN-002`

## Summary

Closed the remaining `ROADMAP.md` final pre-tag checklist rows by:

1. intentionally bumping crate version to `1.0.0`
2. proving release-candidate build from a clean working tree snapshot

## Repository Updates

- `Cargo.toml` version set to `1.0.0`.
- Release-state docs synchronized:
  - `README.md` crate-version line
  - `ROADMAP.md` crate-version header
  - `docs/RELEASE_PROCESS.md` version-policy wording
- `ROADMAP.md` final checklist rows flipped to complete:
  - `Cargo version is bumped intentionally.`
  - `Release candidate is built from a clean working tree.`

## Validation Commands And Results

1. Release-state consistency in primary workspace:

```bash
bash .github/scripts/check-release-state.sh
```

Result: PASS

2. Impacted contract tests in primary workspace:

```bash
cargo test --test release_candidate_gate_contract
cargo test --test release_process_docs_contract
cargo test --test pre_v1_master_checklist_contract
```

Result: PASS (all tests)

3. Clean-tree RC proof in isolated clean clone snapshot (`/private/tmp/kujo_v1u_open_002_clean_clone`):

```bash
git clone . /private/tmp/kujo_v1u_open_002_clean_clone
# copy loop-1 changed files into clone
# create isolated proof commit
bash scripts/release_candidate_gate.sh --roadmap-only
cargo build --release --locked
git status --short
```

Results:

- `release_candidate_gate.sh --roadmap-only`: PASS
- `cargo build --release --locked`: PASS (`Finished release profile`)
- `git status --short`: empty output (clean tree)

## Notes

- `scripts/release_candidate_gate.sh --full` in the clean clone still fails at `cargo fmt --check` on pre-existing formatting drift outside this checklist item's direct scope. This does not change the acceptance target for `V1U-OPEN-002`, which is closure of the two roadmap pre-tag rows.
