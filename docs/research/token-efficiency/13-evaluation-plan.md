# Evaluation and experiments

Every optimization must compare current and variant on identical tasks, model settings, provider, and replay fixtures where possible. Metrics: task success, compile/test success, requirements adherence, security constraints, dispatch correctness, hallucination/error rate, retries, latency, input/output/total tokens, and cost.

## Priority experiments

1. **Ledger hypothesis:** component accounting explains at least 95% of provider input tokens on replayable calls. Baseline: raw request plus usage. Variant: ledger. Success: attribution coverage ≥95%, no payload change.
2. **Skill disclosure hypothesis:** loading only skill core before references cuts selected-skill input by ≥30% with task success loss ≤1 percentage point. Include missing-reference and injection cases.
3. **Handoff hypothesis:** typed references reduce child input by ≥30% with no loss in acceptance/security checks. Compare full parent replay.
4. **Resume hypothesis:** structured state + selective evidence reduces recovery input by ≥30% with equal recovery success and zero stale-source violations.
5. **Tool view hypothesis:** summary/detail tool registration reduces schema input while preserving tool selection and argument validity across providers.
6. **Repository retrieval hypothesis:** changed-file/symbol/dependency retrieval beats full-file baseline on multi-file tasks without missed dependency edits.

Each experiment needs at least easy, realistic, edge, failure, and adversarial tasks, with deterministic offline fixtures and live-provider confirmation where permitted.
