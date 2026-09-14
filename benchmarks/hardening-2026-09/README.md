# September 2026 hardening evaluation fixtures

These fixtures compare pre-pass commit `1a7b6ef` with post-pass commit
`1395f49` using identical workloads.

- `eval.json` is the Kujo Eval suite.
- `context_metrics.kujo` estimates README, INSTALLATION, and ROADMAP context.
- `scale_workload.kujo` provides deterministic large, stress, and scaling work.
- `invalid_source.kujo` is the deliberate parser-failure fixture.

The three document budgets in `eval.json` are evaluator policy, not general Kujo
repository limits. Content checks accompany them so deleting useful guidance
cannot earn a passing score.

Run the complete comparison from the repository root:

```bash
bash scripts/run_hardening_evaluation.sh
```

See `docs/evaluations/SEPTEMBER_2026_HARDENING_EVALUATION.md` for methodology,
limitations, and interpretation.
