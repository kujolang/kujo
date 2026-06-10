# V1U-FINAL-002 Evidence: Release Dry Run From Clean Tree

Date: 2026-05-21
Item: `V1U-FINAL-002`

## Dry-Run Scope

Rehearsed release execution flow from a clean clone without publishing artifacts or pushing tags.

Dry-run workspace: `/private/tmp/kujo_v1u_final_002_dryrun`

## Command Flow And Results

1. Create clean dry-run clone and rehearsal branch:

```bash
git clone . /private/tmp/kujo_v1u_final_002_dryrun
cp Cargo.lock /private/tmp/kujo_v1u_final_002_dryrun/Cargo.lock
git switch -c codex/v1u-final-002-dryrun
```

Result: PASS

2. Validate clean-tree preconditions:

```bash
git status --short
git branch --show-current
```

Result:
- `git status --short`: empty output (clean tree)
- branch: `codex/v1u-final-002-dryrun`

3. Run roadmap readiness precheck:

```bash
bash scripts/release_candidate_gate.sh --roadmap-only
```

Result: PASS

4. Run repeatable local release smoke gate:

```bash
bash scripts/release_gate.sh --minimal
```

Result: PASS

5. Validate release-state file consistency:

```bash
bash .github/scripts/check-release-state.sh
```

Result: PASS

6. Rehearse local tag flow without publication:

```bash
git tag -a v1.0.0-dry-run-20260521 -m "Kujo v1.0.0 dry-run rehearsal"
git tag --list "v1.0.0-dry-run-20260521"
```

Result: PASS (tag created locally and listed; no push/publish step executed)

7. Full RC gate rehearsal (expected failure capture):

```bash
bash scripts/release_candidate_gate.sh --full
```

Result: FAIL (deterministic) at `cargo fmt --check` due pre-existing formatting drift in tracked files (`src/vm.rs`, multiple test files). This is repeatable and already captured as follow-up work in gate evidence loops.

## Dry-Run Determinism Conclusion

- All required dry-run steps were executable from a clean tree with no undocumented manual operations.
- Failure mode for full RC gate is deterministic and reproducible (`cargo fmt --check` drift), not flaky.
- No publish/tag push actions were performed during this rehearsal.
